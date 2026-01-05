use std::{
	collections::HashMap,
	fs::File,
	path::{Path, PathBuf},
	time::{Duration, SystemTime},
};

use anyhow::{bail, Context};
use nitro_config::instance::{make_valid_instance_id, InstanceConfig};
use nitro_plugin::{
	api::wasm::{
		output::WASMPluginOutput,
		sys::{get_os_string, run_command},
		util::get_custom_config,
		WASMPlugin,
	},
	hook::hooks::{ImportInstanceResult, ReplaceInstanceLaunchResult},
	nitro_wasm_plugin,
};
use nitro_shared::{id::InstanceID, loaders::Loader, versions::MinecraftVersionDeser, Side};
use nitro_shared::{
	output::{MessageContents, MessageLevel, NitroOutput},
	util::to_string_json,
};
use serde::Deserialize;
use zip::ZipArchive;

/// Custom field on an instance for the auto-mcs server name
static SERVER_NAME_CONFIG: &str = "auto_mcs_server_name";

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

			config.game_dir = Some(path.to_string_lossy().to_string());
			config.custom_launch = true;
			config.is_editable = false;

			let server_name = config
				.name
				.clone()
				.unwrap_or_else(|| path.file_name().unwrap().to_string_lossy().to_string());

			let id = make_valid_instance_id(&server_name);
			let id = format!("auto-mcs:{id}");

			config.plugin_config.insert(
				SERVER_NAME_CONFIG.into(),
				serde_json::Value::String(server_name),
			);

			instances.insert(InstanceID::from(id), config);
		}

		Ok(instances)
	})?;

	plugin.replace_instance_launch(|arg| {
		if arg.config.source_plugin.is_none_or(|x| x != "auto_mcs") {
			return Ok(None);
		}

		let custom_config = get_custom_config().unwrap_or("{}".into());
		let custom_config: GlobalConfig =
			serde_json::from_str(&custom_config).context("Failed to deserialize custom config")?;
		let auto_mcs_path = custom_config
			.auto_mcs_path
			.context("You must specify a path for the auto-mcs executable in your plugin config")?;
		let auto_mcs_path = PathBuf::from(auto_mcs_path);

		if !auto_mcs_path.exists() {
			bail!("auto-mcs executable does not exist. Did you specify the path correctly?");
		}

		let server_name = arg
			.config
			.plugin_config
			.get(SERVER_NAME_CONFIG)
			.context("Server name is not present in instance")?;
		let serde_json::Value::String(server_name) = server_name else {
			bail!("Server name is not a string");
		};

		let _ = File::create(arg.stdout_path.unwrap());

		// Get the log entries before the spawn to detect which one is the new one
		let log_dir = get_auto_mcs_dir()?.join("Logs/application");
		let original_logs = get_dir_file_timestamps(&log_dir).unwrap_or_default();

		let (_, pid) = run_command(
			auto_mcs_path,
			vec!["--headless", "--launch", &server_name],
			None::<String>,
			None::<String>,
			true,
			true,
			false,
		)
		.context("Failed to spawn auto-mcs server")?;

		let mut o = WASMPluginOutput::new();
		o.display(
			MessageContents::Simple("Checking log file dir".into()),
			MessageLevel::Debug,
		);

		// Keep checking for new log files, then pick the newest one when the entries change
		let log_file_path = loop {
			let logs = get_dir_file_timestamps(&log_dir).unwrap_or_default();
			if original_logs != logs {
				if let Some(result) = logs.into_iter().max_by_key(|x| x.1) {
					break result.0;
				}
			}

			std::thread::sleep(Duration::from_millis(50));
		};

		let log_file_path = log_file_path.to_string_lossy().to_string();

		o.display(
			MessageContents::Simple(format!("Found log file {log_file_path}")),
			MessageLevel::Debug,
		);

		Ok(Some(ReplaceInstanceLaunchResult {
			pid,
			stdout_path: Some(log_file_path),
		}))
	})?;

	plugin.delete_instance(|arg| {
		let server_name = arg
			.config
			.plugin_config
			.get(SERVER_NAME_CONFIG)
			.context("Server name is not present in instance")?;
		let serde_json::Value::String(server_name) = server_name else {
			bail!("Server name is not a string");
		};

		let auto_mcs_dir = get_auto_mcs_dir().context("Failed to get auto-mcs data directory")?;
		let servers_dir = auto_mcs_dir.join("Servers");
		let path = servers_dir.join(server_name);

		if path.exists() {
			std::fs::remove_dir_all(path).context("Failed to delete server")?;
		}

		Ok(())
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
		version: Some(MinecraftVersionDeser::Version(version.into())),
		loader: Some(loader),
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

/// Gets a list of files and their creation times in a directory, sorted by name
pub fn get_dir_file_timestamps(dir: &Path) -> anyhow::Result<Vec<(PathBuf, SystemTime)>> {
	if !dir.exists() {
		return Ok(Vec::new());
	}

	let read = dir.read_dir().context("Failed to read directory")?;

	let entries = read.filter_map(|x| {
		let x = x.ok()?;
		if !x.file_type().ok()?.is_file() {
			return None;
		}

		let time = x.metadata().ok()?.created().ok()?;

		Some((x.path(), time))
	});

	let mut entries: Vec<_> = entries.collect();
	entries.sort_by_cached_key(|x| x.0.clone());

	Ok(entries)
}

#[derive(Deserialize)]
struct GlobalConfig {
	#[serde(default)]
	auto_mcs_path: Option<String>,
}
