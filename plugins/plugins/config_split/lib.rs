use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::Context;
use nitro_config::{instance::InstanceConfig, template::TemplateConfig};
use nitro_plugin::{
	api::wasm::{sys::get_config_dir, WASMPlugin},
	nitro_wasm_plugin,
};
use serde::{de::DeserializeOwned, Serialize};

nitro_wasm_plugin!(main, "config_split");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.add_instances(|_| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("instances");
		if !dir.exists() {
			let _ = std::fs::create_dir_all(&dir);
		}

		let mut configs = get_config_files::<InstanceConfig>(&dir)?;

		for config in configs.values_mut() {
			config.is_editable = true;
			config.is_deletable = true;
		}

		Ok(configs)
	})?;

	plugin.add_templates(|_| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("templates");
		if !dir.exists() {
			let _ = std::fs::create_dir_all(&dir);
		}

		let mut configs = get_config_files::<TemplateConfig>(&dir)?;

		for config in configs.values_mut() {
			config.instance.is_editable = true;
			config.instance.is_deletable = true;
		}

		Ok(configs)
	})?;

	plugin.save_instance_config(|mut arg| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("instances");
		if !dir.exists() {
			let _ = std::fs::create_dir_all(&dir);
		}

		arg.config.remove_plugin_only_fields();

		save_config_file(&dir, &arg.id, arg.config)
	})?;

	plugin.save_template_config(|mut arg| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("templates");
		if !dir.exists() {
			let _ = std::fs::create_dir_all(&dir);
		}

		arg.config.instance.remove_plugin_only_fields();

		save_config_file(&dir, &arg.id, arg.config)
	})?;

	plugin.delete_instance(|arg| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("instances");
		if !dir.exists() {
			return Ok(());
		}

		remove_config_file(&dir, &arg)
	})?;

	plugin.delete_template(|arg| {
		let config_dir = get_config_dir();
		let dir = config_dir.join("templates");
		if !dir.exists() {
			return Ok(());
		}

		remove_config_file(&dir, &arg)
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
		let name = &name[0..name.len() - 5];

		let contents = std::fs::read(entry.path())
			.with_context(|| format!("Failed to read config file '{name}'"))?;
		let contents = serde_json::from_slice(&contents)
			.with_context(|| format!("Failed to read config file '{name}'"))?;

		out.insert(Arc::from(name), contents);
	}

	Ok(out)
}

/// Saves a config file in the given directory
fn save_config_file<S: Serialize>(directory: &Path, id: &str, config: S) -> anyhow::Result<()> {
	let path = directory.join(format!("{id}.json"));

	let data = serde_json::to_vec_pretty(&config)?;

	std::fs::write(path, data).context("Failed to write config file")
}

/// Removes a config file in the given directory
fn remove_config_file(directory: &Path, id: &str) -> anyhow::Result<()> {
	let path = directory.join(format!("{id}.json"));

	std::fs::remove_file(path).context("Failed to remove config file")
}
