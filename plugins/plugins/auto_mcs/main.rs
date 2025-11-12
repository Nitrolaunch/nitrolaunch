use std::{collections::HashMap, fs::File, path::PathBuf};

use anyhow::Context;
use nitro_core::util::{json::to_string_json, versions::MinecraftVersionDeser};
use nitro_plugin::{api::CustomPlugin, hooks::ImportInstanceResult};
use nitro_shared::{loaders::Loader, Side};
use nitrolaunch::config_crate::instance::{CommonInstanceConfig, InstanceConfig};
use zip::ZipArchive;

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("auto_mcs", include_str!("plugin.json"))?;

	plugin.import_instance(|_, arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		let mut zip = ZipArchive::new(File::open(source_path).context("Failed to open instance")?)?;

		// Read the INI file
		let mut ini_file = zip
			.by_name("auto-mcs.ini")
			.context("INI file is missing in instance")?;
		let ini =
			std::io::read_to_string(&mut ini_file).context("Failed to read instance config")?;
		let ini = read_ini(&ini);
		std::mem::drop(ini_file);

		// Write the instance files

		// Extract all the instance files
		if !target_path.exists() {
			std::fs::create_dir_all(&target_path)?;
		}

		zip.extract(&target_path)?;

		// Remove the server.jar to save space
		let server_jar_path = target_path.join("server.jar");
		if server_jar_path.exists() {
			let _ = std::fs::remove_file(server_jar_path);
		}

		let config = create_config(ini).context("Failed to create config")?;

		Ok(ImportInstanceResult {
			format: arg.format,
			config,
		})
	})?;

	Ok(())
}

/// Creates the config for an instance from auto-mcs.ini
fn create_config(mut ini: HashMap<&str, HashMap<&str, &str>>) -> anyhow::Result<InstanceConfig> {
	let mut general = ini
		.remove("general")
		.context("General info section missing")?;
	let name = general.remove("serverName");
	let version = general
		.remove("serverVersion")
		.context("Minecraft version missing")?;
	let loader = match general.remove("serverType") {
		None | Some("vanilla") => Loader::Vanilla,
		Some("paper") => Loader::Paper,
		Some("fabric") => Loader::Fabric,
		Some("forge") => Loader::Forge,
		Some("purpur") => Loader::Purpur,
		Some(other) => Loader::parse_from_str(other),
	};

	let loader = to_string_json(&loader);

	Ok(InstanceConfig {
		name: name.map(|x| x.to_string()),
		side: Some(Side::Server),
		common: CommonInstanceConfig {
			version: Some(MinecraftVersionDeser::Version(version.into())),
			loader: Some(loader),
			..Default::default()
		},
		..Default::default()
	})
}

/// Reads the auto-mcs.ini file for a server
fn read_ini(contents: &str) -> HashMap<&str, HashMap<&str, &str>> {
	let mut sections: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
	let mut current_section = "global";

	for line in contents.lines() {
		if let Some((key, value)) = line.split_once(" = ") {
			// Remove quotes from strings
			let value = value
				.strip_prefix("'")
				.unwrap_or(value)
				.strip_suffix("'")
				.unwrap_or(value);

			sections
				.entry(current_section)
				.or_default()
				.insert(key, value);
		} else if line.starts_with("[") {
			current_section = &line[1..line.len() - 1]
		}
	}

	sections
}
