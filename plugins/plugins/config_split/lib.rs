use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::Context;
use nitro_plugin::{
	api::wasm::{sys::get_config_dir, WASMPlugin},
	nitro_wasm_plugin,
};
use serde::de::DeserializeOwned;

nitro_wasm_plugin!(main, "config_split");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.add_instances(|_| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("instances");
		if !dir.exists() {
			let _ = std::fs::create_dir_all(&dir);
		}

		get_config_files(&dir)
	})?;
	plugin.add_templates(|_| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("templates");
		if !dir.exists() {
			let _ = std::fs::create_dir_all(&dir);
		}

		get_config_files(&dir)
	})?;

	Ok(())
}

/// Gets config files from the given directory
fn get_config_files<D: DeserializeOwned>(directory: &Path) -> anyhow::Result<HashMap<Arc<str>, D>> {
	let reader = directory.read_dir().context("Failed to read directory")?;

	let mut out = HashMap::with_capacity(reader.size_hint().0);
	for entry in reader {
		let entry = entry.context("Failed to read directory entry")?;
		if entry
			.file_type()
			.context("Failed to get entry file type")?
			.is_dir()
		{
			continue;
		}

		let name = entry.file_name().to_string_lossy().to_string();
		if !name.ends_with(".json") {
			continue;
		}
		let name = &name[0..name.len() - 6];

		let contents = std::fs::read(entry.path())
			.with_context(|| format!("Failed to read config file '{name}'"))?;
		let contents = serde_json::from_slice(&contents)
			.with_context(|| format!("Failed to read config file '{name}'"))?;

		out.insert(Arc::from(name), contents);
	}

	Ok(out)
}
