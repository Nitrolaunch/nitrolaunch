use std::{collections::HashMap, fs::File, path::PathBuf};

use anyhow::Context;
use nitro_core::util::{json::to_string_json, versions::MinecraftVersionDeser};
use nitro_plugin::{api::CustomPlugin, hook::hooks::ImportInstanceResult};
use nitro_shared::{
	id::InstanceID,
	loaders::Loader,
	output::{MessageContents, MessageLevel, NitroOutput},
	Side,
};
use nitrolaunch::config_crate::instance::{
	make_valid_instance_id, CommonInstanceConfig, InstanceConfig,
};
use zip::ZipArchive;

#[cfg(not(target_os = "linux"))]
static INI_FILENAME: &str = "auto-mcs.ini";
#[cfg(target_os = "linux")]
static INI_FILENAME: &str = ".auto-mcs.ini";

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("auto_mcs", include_str!("plugin.json"))?;

	plugin.import_instance(|_, arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		let mut zip = ZipArchive::new(File::open(source_path).context("Failed to open instance")?)?;

		// Read the INI file
		let mut ini_file = zip
			.by_name(INI_FILENAME)
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

	plugin.add_instances(|mut ctx, _| {
		let auto_mcs_dir = get_auto_mcs_dir().context("Failed to get auto-mcs data directory")?;
		let servers_dir = auto_mcs_dir.join("Servers");

		let mut instances = HashMap::new();
		for entry in servers_dir.read_dir()? {
			let Ok(entry) = entry else {
				ctx.get_output().display(
					MessageContents::Error("Failed to load auto-mcs server".into()),
					MessageLevel::Important,
				);
				continue;
			};

			let path = entry.path();

			// Read the INI
			let ini_path = path.join(INI_FILENAME);
			let Ok(ini) = std::fs::read_to_string(ini_path) else {
				ctx.get_output().display(
					MessageContents::Error(format!("Failed to load auto-mcs server ({path:?})")),
					MessageLevel::Important,
				);
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
	#[cfg(target_os = "linux")]
	let data_folder = format!("{}/.auto-mcs", std::env::var("HOME")?);
	#[cfg(target_os = "windows")]
	let data_folder = format!("{}/Roaming/.auto-mcs", std::env::var("%APPDATA%")?);
	#[cfg(target_os = "macos")]
	let data_folder = format!(
		"{}/Library/Application Support/.auto-mcs",
		std::env::var("HOME")?
	);

	Ok(PathBuf::from(data_folder))
}
