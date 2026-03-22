use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::hook::call::HookHandle;
use crate::hook::call::HookHandles;
use crate::hook::hooks::OnLoad;
use crate::hook::hooks::StartWorker;
use crate::hook::wasm::loader::WASMLoader;
use crate::hook::Hook;
use crate::plugin::PluginProvidedSubcommand;
use crate::plugin::{HookPriority, Plugin, DEFAULT_PROTOCOL_VERSION, NEWEST_PROTOCOL_VERSION};
use crate::PluginPaths;
use anyhow::{bail, Context};
use itertools::Itertools;
use nitro_config::instance::InstanceConfig;
use nitro_config::template::TemplateConfig;
use nitro_shared::output::NitroOutput;
use tokio::sync::Mutex;

/// A manager for plugins that is used to call their hooks.
/// Does not handle actually loading the plugins from files
pub struct CorePluginManager {
	plugins: Vec<Plugin>,
	plugin_list: Vec<String>,
	nitro_version: Option<&'static str>,
	wasm_loader: Arc<Mutex<WASMLoader>>,
	instances: Option<Arc<HashMap<String, InstanceConfig>>>,
	templates: Option<Arc<HashMap<String, TemplateConfig>>>,
}

impl CorePluginManager {
	/// Construct a new PluginManager
	pub fn new(paths: &PluginPaths) -> Self {
		Self {
			plugins: Vec::new(),
			plugin_list: Vec::new(),
			nitro_version: None,
			wasm_loader: Arc::new(Mutex::new(WASMLoader::new(&paths.data_dir))),
			instances: None,
			templates: None,
		}
	}

	/// Set the Nitrolaunch version of the manager
	pub fn set_nitro_version(&mut self, version: &'static str) {
		self.nitro_version = Some(version);
	}

	/// Set the WASM loader of the manager
	pub fn set_wasm_loader(&mut self, loader: Arc<Mutex<WASMLoader>>) {
		self.wasm_loader = loader;
	}

	/// Add a plugin to the manager
	pub async fn add_plugin(
		&mut self,
		mut plugin: Plugin,
		paths: &PluginPaths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		// Check the protocol version
		if plugin
			.get_manifest()
			.protocol_version
			.unwrap_or(DEFAULT_PROTOCOL_VERSION)
			> NEWEST_PROTOCOL_VERSION
		{
			bail!("Plugin has a newer protocol version than Nitrolaunch");
		}

		// Update the plugin list
		self.plugin_list.push(plugin.get_id().clone());

		// Call the on_load hook
		let result = plugin
			.call_hook(
				&OnLoad,
				&(),
				paths,
				self.nitro_version,
				&self.plugin_list,
				self.wasm_loader.clone(),
				self.instances.as_ref(),
				self.templates.as_ref(),
				o,
			)
			.await
			.context("Failed to call on_load hook of plugin")?;
		if let Some(result) = result {
			result.result(o).await?;
		}

		// Call the start_worker hook
		let worker_handle = plugin
			.call_hook(
				&StartWorker,
				&(),
				paths,
				self.nitro_version,
				&self.plugin_list,
				self.wasm_loader.clone(),
				self.instances.as_ref(),
				self.templates.as_ref(),
				o,
			)
			.await
			.context("Failed to call start_worker hook of plugin")?;
		if let Some(worker_handle) = worker_handle {
			plugin
				.set_worker(worker_handle)
				.await
				.context("Failed to set plugin worker")?;
		}

		self.plugins.push(plugin);

		Ok(())
	}

	/// Call a plugin hook on the manager and collects the results into a Vec
	pub async fn call_hook<H: Hook>(
		&self,
		hook: H,
		arg: &H::Arg,
		paths: &PluginPaths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<HookHandles<H>> {
		let mut out = VecDeque::new();
		for plugin in self.plugins.iter().sorted_by_key(|x| PluginSort {
			priority: x.get_hook_priority(&hook),
			id: x.get_id().clone(),
		}) {
			let result = plugin
				.call_hook(
					&hook,
					arg,
					paths,
					self.nitro_version,
					&self.plugin_list,
					self.wasm_loader.clone(),
					self.instances.as_ref(),
					self.templates.as_ref(),
					o,
				)
				.await
				.with_context(|| format!("Hook failed for plugin {}", plugin.get_id()))?;
			out.extend(result);
		}

		let handles = HookHandles::new(out, o)
			.await
			.context("Failed to start hook handles")?;

		Ok(handles)
	}

	/// Call a plugin hook on the manager on a specific plugin
	pub async fn call_hook_on_plugin<H: Hook>(
		&self,
		hook: H,
		plugin_id: &str,
		arg: &H::Arg,
		paths: &PluginPaths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<HookHandle<H>>> {
		for plugin in &self.plugins {
			if plugin.get_id() == plugin_id {
				let result = plugin
					.call_hook(
						&hook,
						arg,
						paths,
						self.nitro_version,
						&self.plugin_list,
						self.wasm_loader.clone(),
						self.instances.as_ref(),
						self.templates.as_ref(),
						o,
					)
					.await
					.context("Plugin hook failed")?;
				return Ok(result);
			}
		}

		bail!("No plugin found that matched the given ID")
	}

	/// Iterate over the plugins
	pub fn iter_plugins(&self) -> impl Iterator<Item = &Plugin> {
		self.plugins.iter()
	}

	/// Checks whether the given plugin is present and enabled in the manager
	pub fn has_plugin(&self, plugin_id: &str) -> bool {
		self.plugin_list.iter().any(|x| x == plugin_id)
	}

	/// Gets the plugin to use for a subcommand. Returns none if no plugin provides that subcommand
	pub fn get_subcommand(&self, subcommand: &str, supercommand: Option<&str>) -> Option<String> {
		self.iter_plugins().find(|x| {
			x.get_manifest()
				.subcommands
				.iter()
				.any(|x| {
					if x.0 != subcommand {
						return false;
					}

					if let Some(supercommand2) = supercommand {
						matches!(x.1, PluginProvidedSubcommand::Specific { supercommand, .. } if supercommand == supercommand2)
					} else {
						matches!(x.1, PluginProvidedSubcommand::Global(..))
					}
				})
		})
		.map(|x| x.get_id().clone())
	}

	/// Sets the instance and template list for the manager to pass to plugins
	pub fn set_instances_and_templates(
		&mut self,
		instances: HashMap<String, InstanceConfig>,
		templates: HashMap<String, TemplateConfig>,
	) {
		self.instances = Some(Arc::new(instances));
		self.templates = Some(Arc::new(templates));
	}
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct PluginSort {
	priority: HookPriority,
	id: String,
}
