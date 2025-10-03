use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::bail;
use anyhow::Context;
use nitro_core::Paths;
use nitro_shared::output::NitroOutput;
use serde::{Deserialize, Deserializer};
use tokio::sync::Mutex;

use crate::hook_call::HookCallArg;
use crate::hooks::Hook;
use crate::hooks::StartWorker;
use crate::HookHandle;

/// The newest protocol version for plugin communication
pub const NEWEST_PROTOCOL_VERSION: u16 = 3;
/// The default protocol version used for compatability
pub const DEFAULT_PROTOCOL_VERSION: u16 = 1;
/// Token used for file replacement in hook handlers
pub static FILE_REPLACEMENT_TOKEN: &str = "$file:";

/// A plugin
pub struct Plugin {
	/// The plugin's ID
	id: String,
	/// The plugin's manifest
	manifest: PluginManifest,
	/// The custom config for the plugin, serialized from JSON
	custom_config: Option<String>,
	/// The working directory for the plugin
	working_dir: Option<PathBuf>,
	/// The persistent state of the plugin
	persistence: Arc<Mutex<PluginPersistence>>,
}

impl Plugin {
	/// Create a new plugin from an ID and manifest
	pub fn new(id: String, manifest: PluginManifest) -> Self {
		Self {
			id,
			manifest,
			custom_config: None,
			working_dir: None,
			persistence: Arc::new(Mutex::new(PluginPersistence::new())),
		}
	}

	/// Get the ID of the plugin
	pub fn get_id(&self) -> &String {
		&self.id
	}

	/// Get the manifest of the plugin
	pub fn get_manifest(&self) -> &PluginManifest {
		&self.manifest
	}

	/// Call a hook on the plugin
	pub async fn call_hook<H: Hook>(
		&self,
		hook: &H,
		arg: &H::Arg,
		paths: &Paths,
		nitro_version: Option<&str>,
		plugin_list: &[String],
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<HookHandle<H>>> {
		let Some(handler) = self.manifest.hooks.get(hook.get_name()) else {
			return Ok(None);
		};

		self.call_hook_handler(hook, handler, arg, paths, nitro_version, plugin_list, o)
			.await
	}

	/// Call a hook handler on the plugin
	async fn call_hook_handler<H: Hook>(
		&self,
		hook: &H,
		handler: &HookHandler,
		arg: &H::Arg,
		paths: &Paths,
		nitro_version: Option<&str>,
		plugin_list: &[String],
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<HookHandle<H>>> {
		match handler {
			HookHandler::Execute {
				executable,
				args,
				priority: _,
			} => {
				let arg = HookCallArg {
					cmd: executable,
					arg,
					additional_args: args,
					working_dir: self.working_dir.as_deref(),
					use_base64: !self.manifest.raw_transfer,
					custom_config: self.custom_config.clone(),
					persistence: self.persistence.clone(),
					paths,
					nitro_version,
					plugin_id: &self.id,
					plugin_list,
					protocol_version: self
						.manifest
						.protocol_version
						.unwrap_or(DEFAULT_PROTOCOL_VERSION),
				};
				hook.call(arg, o).await.map(Some)
			}
			HookHandler::Constant {
				constant,
				priority: _,
			} => {
				// Replace file tokens
				let mut value = constant.clone();

				replace_file_tokens(&mut value, &self.working_dir, false)?;

				Ok(Some(HookHandle::constant(
					serde_json::from_value(value)?,
					self.id.clone(),
				)))
			}
			HookHandler::File { file, priority: _ } => {
				let Some(working_dir) = &self.working_dir else {
					bail!("Plugin does not have a directory for the file hook handler to look in");
				};

				let path = working_dir.join(file);
				let contents = std::fs::read_to_string(path)
					.context("Failed to read hook result from file")?;

				// Try to read the result with quotes wrapping it if it doesn't deserialize properly the first time
				let result = match serde_json::from_str(&contents) {
					Ok(result) => result,
					Err(_) => {
						let sanitized = format!("\"{}\"", contents.replace("\"", "\\\""))
							.replace("\t", "\\t")
							.replace("\n", "\\n");
						match serde_json::from_str(&sanitized) {
							Ok(result) => result,
							Err(e) => bail!("Failed to deserialize hook result: {e}"),
						}
					}
				};

				Ok(Some(HookHandle::constant(result, self.id.clone())))
			}
			HookHandler::Match {
				property,
				cases,
				priority: _,
			} => {
				let arg2 = serde_json::to_value(arg)?;
				let lhs = if let Some(property) = property {
					let arg2 = arg2.as_object().context(
						"Hook argument is not an object, so a property cannot be matched",
					)?;
					arg2.get(property)
						.context("Property does not exist on hook argument")
						.cloned()?
				} else {
					arg2
				};
				let lhs = serde_json::to_string(&lhs)?;

				for (case, handler) in cases.iter() {
					if &lhs == case {
						return Box::pin(self.call_hook_handler(
							hook,
							handler,
							arg,
							paths,
							nitro_version,
							plugin_list,
							o,
						))
						.await;
					}
				}

				Ok(None)
			}
			HookHandler::Native {
				function,
				priority: _,
			} => {
				let arg = serde_json::to_value(arg)
					.context("Failed to serialize native hook argument")?;
				let result = function
					.call(arg)
					.await
					.context("Native hook handler failed")?;
				let result = serde_json::from_value(result)
					.context("Failed to deserialize native hook result")?;
				Ok(Some(HookHandle::constant(result, self.id.clone())))
			}
		}
	}

	/// Set the custom config of the plugin
	pub fn set_custom_config(&mut self, config: serde_json::Value) -> anyhow::Result<()> {
		let serialized =
			serde_json::to_string(&config).context("Failed to serialize custom plugin config")?;
		self.custom_config = Some(serialized);
		Ok(())
	}

	/// Set the working dir of the plugin
	pub fn set_working_dir(&mut self, dir: PathBuf) {
		self.working_dir = Some(dir);
	}

	/// Set the plugin's worker handle
	pub async fn set_worker(&mut self, worker: HookHandle<StartWorker>) -> anyhow::Result<()> {
		let mut lock = self.persistence.lock().await;
		lock.worker = Some(worker);

		Ok(())
	}

	/// Get the priority of the given hook
	pub fn get_hook_priority<H: Hook>(&self, hook: &H) -> HookPriority {
		let Some(handler) = self.manifest.hooks.get(hook.get_name()) else {
			return HookPriority::Any;
		};
		match handler {
			HookHandler::Execute { priority, .. }
			| HookHandler::Constant { priority, .. }
			| HookHandler::File { priority, .. }
			| HookHandler::Match { priority, .. }
			| HookHandler::Native { priority, .. } => *priority,
		}
	}
}

/// The manifest for a plugin that describes how it works
#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct PluginManifest {
	/// The display name of the plugin
	pub name: Option<String>,
	/// The short description of the plugin
	pub description: Option<String>,
	/// The Nitrolaunch version this plugin is for
	#[serde(alias = "mcvm_version")]
	pub nitro_version: Option<String>,
	/// The hook handlers for the plugin
	pub hooks: HashMap<String, HookHandler>,
	/// The subcommands the plugin provides
	pub subcommands: HashMap<String, PluginProvidedSubcommand>,
	/// Plugins that this plugin depends on
	pub dependencies: Vec<String>,
	/// Message to display when the plugin is installed
	pub install_message: Option<String>,
	/// The protocol version of the plugin
	pub protocol_version: Option<u16>,
	/// Whether to disable base64 encoding in the protocol
	pub raw_transfer: bool,
}

impl PluginManifest {
	/// Create a new PluginManifest
	pub fn new() -> Self {
		Self::default()
	}
}

/// A CLI subcommand provided by a plugin
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum PluginProvidedSubcommand {
	/// A root-level subcommand, containing the description
	Global(String),
	/// A subsubcommand
	Specific {
		/// The command to be under
		supercommand: String,
		/// The description
		description: String,
	},
}

/// A handler for a single hook that a plugin uses
#[derive(Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "snake_case")]
pub enum HookHandler {
	/// Handle this hook by running an executable
	Execute {
		/// The executable to run
		executable: String,
		/// Arguments for the executable
		#[serde(default)]
		args: Vec<String>,
		/// The priority for the hook
		#[serde(default)]
		priority: HookPriority,
	},
	/// Handle this hook by returning a constant result
	Constant {
		/// The constant result
		constant: serde_json::Value,
		/// The priority for the hook
		#[serde(default)]
		priority: HookPriority,
	},
	/// Handle this hook by getting the contents of a file
	File {
		/// The path to the file, relative to the plugin directory
		file: String,
		/// The priority for the hook
		#[serde(default)]
		priority: HookPriority,
	},
	/// Match against the argument to handle the hook differently
	Match {
		/// The property to match against
		#[serde(default)]
		property: Option<String>,
		/// The cases of the match
		cases: HashMap<String, Box<HookHandler>>,
		/// The priority for the hook
		#[serde(default)]
		priority: HookPriority,
	},
	/// Handle this hook with a native function call
	Native {
		/// The function to handle the hook
		#[serde(deserialize_with = "deserialize_native_function")]
		function: Arc<dyn NativeHookHandler>,
		/// The priority for the hook
		#[serde(default)]
		priority: HookPriority,
	},
}

impl Debug for HookHandler {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "HookHandler")
	}
}

/// Priority for a hook
#[derive(Deserialize, PartialEq, PartialOrd, Eq, Ord, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum HookPriority {
	/// The plugin will try to run before other ones
	First,
	/// The plugin will run at any time, usually in the middle
	#[default]
	Any,
	/// The plugin will try to run after other ones
	Last,
}

/// Deserialize function for the native hook. No plugin manifests should ever use this,
/// so just return a function that does nothing.
fn deserialize_native_function<'de, D>(_: D) -> Result<Arc<dyn NativeHookHandler>, D::Error>
where
	D: Deserializer<'de>,
{
	Ok(Arc::new(NoneHookHandler))
}

/// Trait for native plugin hook handlers
#[async_trait::async_trait]
pub trait NativeHookHandler: Send + Sync {
	/// Call the hook
	async fn call(&self, arg: serde_json::Value) -> anyhow::Result<serde_json::Value>;
}

/// Used for deserialization
struct NoneHookHandler;

#[async_trait::async_trait]
impl NativeHookHandler for NoneHookHandler {
	async fn call(&self, arg: serde_json::Value) -> anyhow::Result<serde_json::Value> {
		let _ = arg;
		Ok(serde_json::Value::Null)
	}
}

/// Persistent state for plugins
pub struct PluginPersistence {
	/// The persistent state of the plugin
	pub state: serde_json::Value,
	/// The long-running plugin worker
	pub worker: Option<HookHandle<StartWorker>>,
}

impl Default for PluginPersistence {
	fn default() -> Self {
		Self::new()
	}
}

impl PluginPersistence {
	/// Initalize the persistent plugin state
	pub fn new() -> Self {
		Self {
			state: serde_json::Value::Null,
			worker: None,
		}
	}
}

/// Replaces file tokens in the string values of JSON value
fn replace_file_tokens(
	value: &mut serde_json::Value,
	working_dir: &Option<PathBuf>,
	test: bool,
) -> anyhow::Result<()> {
	match value {
		serde_json::Value::Array(values) => {
			for value in values {
				replace_file_tokens(value, working_dir, test)?;
			}
		}
		serde_json::Value::Object(props) => {
			for prop in props.values_mut() {
				replace_file_tokens(prop, working_dir, test)?;
			}
		}
		serde_json::Value::String(value) => {
			if let Some(path) = value.strip_prefix(FILE_REPLACEMENT_TOKEN) {
				if test {
					*value = "test".into();
					return Ok(());
				}

				let Some(working_dir) = working_dir else {
					bail!("Plugin does not have a directory for the file hook handler to look in");
				};

				let path = working_dir.join(path);
				let contents = std::fs::read_to_string(path)
					.context("Failed to read hook result from file")?;

				*value = contents;
			}
		}
		_ => {}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_file_token_replacement() {
		let mut json = json!([{
			"foo": "bar",
			"baz": format!("{FILE_REPLACEMENT_TOKEN}foobar")
		}]);

		replace_file_tokens(&mut json, &None, true).unwrap();

		let expected = json!([{
			"foo": "bar",
			"baz": "test"
		}]);
		assert_eq!(json, expected);
	}
}
