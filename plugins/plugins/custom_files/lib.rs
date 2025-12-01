use std::path::PathBuf;

use anyhow::Context;
use nitro_plugin::api::wasm::sys::update_hardlink;
use nitro_plugin::api::wasm::WASMPlugin;
use nitro_plugin::hook::hooks::OnInstanceSetupResult;
use nitro_plugin::nitro_wasm_plugin;
use serde::{Deserialize, Serialize};

nitro_wasm_plugin!(main, "custom_files");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.on_instance_setup(|args| {
		let Some(game_dir) = args.game_dir else {
			return Ok(OnInstanceSetupResult::default());
		};

		let Some(config) = args.config.plugin_config.get("custom_files") else {
			return Ok(OnInstanceSetupResult::default());
		};
		let config: Config = serde_json::from_value(config.clone())
			.context("Configuration is incorrectly formatted")?;

		let game_dir = PathBuf::from(game_dir);

		// Copy all of the files
		for file in config.files {
			let src = PathBuf::from(shellexpand::tilde(&file.source).to_string());
			let target = game_dir.join(PathBuf::from(file.target));

			if let Some(parent) = target.parent() {
				std::fs::create_dir_all(&parent)
					.context("Failed to create leading directories to file")?;
			}

			if file.link {
				update_hardlink(&src, &target)
					.with_context(|| format!("Failed to link custom file {}", file.source))?;
			} else {
				std::fs::copy(src, target)
					.with_context(|| format!("Failed to link custom file {}", file.source))?;
			}
		}

		Ok(OnInstanceSetupResult::default())
	})?;

	Ok(())
}

#[derive(Serialize, Deserialize)]
struct Config {
	files: Vec<File>,
}

#[derive(Serialize, Deserialize)]
struct File {
	source: String,
	target: String,
	link: bool,
}
