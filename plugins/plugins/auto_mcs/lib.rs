use std::{collections::HashMap, fs::File, path::PathBuf};

use anyhow::Context;
use nitro_config::instance::{make_valid_instance_id, CommonInstanceConfig, InstanceConfig};
use nitro_plugin::{
	api::wasm::{sys::get_os_string, WASMPlugin},
	hook::hooks::ImportInstanceResult,
	nitro_wasm_plugin,
};
use nitro_shared::util::to_string_json;
use nitro_shared::{id::InstanceID, loaders::Loader, versions::MinecraftVersionDeser, Side};
use zip::ZipArchive;

nitro_wasm_plugin!(main, "auto_mcs");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.import_instance(|arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		let mut zip = ZipArchive::new(File::open(source_path).context("Failed to open instance")?)?;

		// Read the INI file
		let mut ini_file = zip
			.by_name(ini_filename())
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

	plugin.add_instances(|_| {
		let auto_mcs_dir = get_auto_mcs_dir().context("Failed to get auto-mcs data directory")?;
		let servers_dir = auto_mcs_dir.join("Servers");

		let mut instances = HashMap::new();
		for entry in servers_dir.read_dir()? {
			let Ok(entry) = entry else {
				eprintln!("Failed to load auto-mcs server");
				continue;
			};

			let path = entry.path();

			// Read the INI
			let ini_path = path.join(ini_filename());
			let Ok(ini) = std::fs::read_to_string(ini_path) else {
				eprintln!("Failed to load auto-mcs server ({path:?})");
				continue;
			};
			let ini = read_ini(&ini);

			let Ok(mut config) = create_config(ini) else {
				continue;
			};

			config.common.game_dir = Some(path.to_string_lossy().to_string());

			let id = make_valid_instance_id(
				&config
					.name
					.clone()
					.unwrap_or_else(|| path.file_name().unwrap().to_string_lossy().to_string()),
			);
			let id = format!("auto-mcs-{id}");

			instances.insert(InstanceID::from(id), config);
		}

		Ok(instances)
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

/// Gets the filename for the .ini file in the server dir
fn ini_filename() -> &'static str {
	match get_os_string().as_str() {
		"linux" => ".auto-mcs.ini",
		_ => "auto-mcs-ini",
	}
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

fn get_auto_mcs_dir() -> anyhow::Result<PathBuf> {
	let data_folder = match get_os_string().as_str() {
		"linux" => format!("{}/.auto-mcs", std::env::var("HOME")?),
		"windows" => format!("{}/Roaming/.auto-mcs", std::env::var("%APPDATA%")?),
		"macos" => format!(
			"{}/Library/Application Support/.auto-mcs",
			std::env::var("HOME")?
		),
		_ => format!("{}/.auto-mcs", std::env::var("HOME")?),
	};

	Ok(PathBuf::from(data_folder))
}
