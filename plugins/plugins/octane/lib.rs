use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use nitro_config::instance::WrapperCommand;
use nitro_plugin::api::wasm::WASMPlugin;
use nitro_plugin::api::wasm::output::WASMPluginOutput;
use nitro_plugin::api::wasm::sys::get_os_string;
use nitro_plugin::api::wasm::util::{get_persistent_state, set_persistent_state};
use nitro_plugin::hook::hooks::OnInstanceSetupResult;
use nitro_plugin::nitro_wasm_plugin;
use nitro_shared::loaders::Loader;
use nitro_shared::output::{MessageContents, NitroOutput};
use serde::{Deserialize, Serialize};

use crate::args::ArgsPreset;
use crate::cds::{
	create_dump, get_cached_paths, get_dump_use_args, get_list_creation_args, hash_classpath,
};

/// JVM argument presets
mod args;
/// Class-loading cache
mod cds;

nitro_wasm_plugin!(main, "octane");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.on_instance_setup(|arg| {
		let mut jvm_args = Vec::new();

		// Presets
		if let Some(preset) = arg.config.plugin_config.get("octane_preset") {
			if !preset.is_null() {
				let preset: ArgsPreset = serde_json::from_value(preset.clone())
					.context("Failed to deserialize preset")?;
				jvm_args.extend(preset.generate_args());
			}
		}

		let wrapper = if let Some(priority) = arg.config.plugin_config.get("octane_priority") {
			if !priority.is_null() {
				let priority: Priority = serde_json::from_value(priority.clone())
					.context("Failed to deserialize priority")?;
				priority.to_wrapper()
			} else {
				None
			}
		} else {
			None
		};

		Ok(OnInstanceSetupResult {
			jvm_args,
			wrappers: Vec::from_iter(wrapper),
			..Default::default()
		})
	})?;

	plugin.after_instance_setup(|arg| {
		if arg.loader != Loader::Vanilla {
			return Ok(OnInstanceSetupResult::default());
		}

		let Some(cds) = arg.config.plugin_config.get("cds") else {
			return Ok(OnInstanceSetupResult::default());
		};

		if *cds != serde_json::Value::Bool(true) {
			return Ok(OnInstanceSetupResult::default());
		};

		let Some(classpath) = arg.classpath else {
			return Ok(OnInstanceSetupResult::default());
		};

		let mut o = WASMPluginOutput::new();

		let hash = hash_classpath(&classpath);
		let (list_path, dump_path) = get_cached_paths(&hash);

		// Pick whether to use the dump, create the list, or neither
		let jvm_args = if dump_path.exists() {
			o.debug(MessageContents::Simple(format!(
				"Using CDS with hash {hash}"
			)));

			get_dump_use_args(&dump_path)
		} else if !list_path.exists() {
			o.debug(MessageContents::Simple(format!(
				"Creating CDS class list with hash {hash}"
			)));

			if let Some(parent) = list_path.parent() {
				let _ = std::fs::create_dir_all(parent);
			}
			get_list_creation_args(&list_path)
		} else {
			Vec::new()
		};

		// Save the JVM path in state
		let mut state: PersistentState = get_persistent_state().unwrap_or_default();
		state.cds_context.insert(
			arg.id,
			CDSContext {
				jvm_path: arg.jvm_path.into(),
			},
		);
		set_persistent_state(&state);

		Ok(OnInstanceSetupResult {
			jvm_args,
			..Default::default()
		})
	})?;

	plugin.while_instance_launch(|arg| {
		let Some(classpath) = arg.classpath else {
			return Ok(());
		};

		let state: PersistentState = get_persistent_state().unwrap_or_default();
		let Some(context) = state.cds_context.get(&arg.id) else {
			return Ok(());
		};

		let mut o = WASMPluginOutput::new();

		let hash = hash_classpath(&classpath);
		let (list_path, dump_path) = get_cached_paths(&hash);

		// Create the CDS dump if it does not exist, after some heuristic to determine if enough of the class list has been created
		if !dump_path.exists() && list_path.exists() {
			std::thread::sleep(Duration::from_secs(20));

			o.debug(MessageContents::Simple("Dumping CDS classes".into()));
			if let Err(e) = create_dump(&list_path, &dump_path, classpath, &context.jvm_path) {
				o.debug(MessageContents::Error(format!(
					"Failed to create CDS dump: {e}"
				)));
			}
		}

		Ok(())
	})?;

	Ok(())
}

#[derive(Serialize, Deserialize, Default)]
struct PersistentState {
	/// Map of instance IDs to context about the launch for CDS
	cds_context: HashMap<String, CDSContext>,
}

#[derive(Serialize, Deserialize)]
struct CDSContext {
	jvm_path: PathBuf,
}

/// Process priority preset
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Priority {
	None,
	High,
	Background,
}

impl Priority {
	fn to_wrapper(&self) -> Option<WrapperCommand> {
		match get_os_string().as_str() {
			"windows" => self.to_windows_command(),
			"linux" | "macos" => self.to_nice_command(),
			_ => None,
		}
	}

	fn to_nice_command(&self) -> Option<WrapperCommand> {
		let level = match self {
			Self::None => return None,
			Self::High => 0,
			Self::Background => 5,
		};

		Some(WrapperCommand {
			cmd: "nice".into(),
			args: vec!["-n".into(), level.to_string()],
		})
	}

	fn to_windows_command(&self) -> Option<WrapperCommand> {
		let flag = match self {
			Self::None => return None,
			Self::High => "/high",
			Self::Background => "/belownormal",
		};

		Some(WrapperCommand {
			cmd: "start".into(),
			args: vec![flag.into()],
		})
	}
}
