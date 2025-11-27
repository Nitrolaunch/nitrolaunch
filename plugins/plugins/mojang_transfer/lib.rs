use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Context};
use nitro_config::instance::{
	make_valid_instance_id, Args, CommonInstanceConfig, InstanceConfig, LaunchArgs, LaunchConfig,
};
use nitro_plugin::{
	api::wasm::{sys::get_os_string, WASMPlugin},
	hook::hooks::MigrateInstancesResult,
	nitro_wasm_plugin,
};
use nitro_shared::{
	versions::{MinecraftLatestVersion, MinecraftVersionDeser},
	Side,
};
use serde::{Deserialize, Serialize};

nitro_wasm_plugin!(main, "mojang_transfer");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.migrate_instances(|arg| {
		let os = get_os_string();
		let data_folder = match os.as_str() {
			"linux" => format!("{}/.local/share/.minecraft", std::env::var("HOME")?),
			"windows" => format!("{}/Roaming/.minecraft", std::env::var("%APPDATA%")?),
			"macos" => format!(
				"{}/Library/Application Support/.minecraft",
				std::env::var("HOME")?
			),
			_ => bail!("Unsupported OS"),
		};

		let data_folder = PathBuf::from(data_folder);
		let launcher_profiles = data_folder.join("launcher_profiles.json");
		if !launcher_profiles.exists() {
			return Ok(MigrateInstancesResult {
				format: arg,
				..Default::default()
			});
		}

		let data = match std::fs::read(&launcher_profiles) {
			Ok(data) => data,
			Err(e) => bail!("Failed to read launcher profiles: {e:#?}"),
		};

		let profiles: LauncherProfiles =
			serde_json::from_slice(&data).context("Failed to deserialize launcher profiles")?;

		let mut instances = HashMap::new();

		for profile in profiles.profiles.into_values() {
			let id = make_valid_instance_id(&profile.name);

			let id = if instances.contains_key(&id) {
				id + "2"
			} else {
				id
			};

			let config = create_config(profile).context("Failed to create config")?;

			instances.insert(id.clone(), config);
		}

		Ok(MigrateInstancesResult {
			format: arg,
			instances,
			packages: HashMap::new(),
		})
	})?;

	Ok(())
}

/// Creates the config for an instance from metadata
fn create_config(profile: Profile) -> anyhow::Result<InstanceConfig> {
	let version = match profile.kind {
		ProfileType::Custom => MinecraftVersionDeser::Version(profile.last_version_id.into()),
		ProfileType::LatestRelease => {
			MinecraftVersionDeser::Latest(MinecraftLatestVersion::Release)
		}
		ProfileType::LatestSnapshot => {
			MinecraftVersionDeser::Latest(MinecraftLatestVersion::Snapshot)
		}
	};

	let args = if profile.java_args.is_empty() {
		LaunchArgs::default()
	} else {
		LaunchArgs {
			jvm: Args::String(profile.java_args),
			..Default::default()
		}
	};

	Ok(InstanceConfig {
		name: Some(profile.name),
		side: Some(Side::Client),
		common: CommonInstanceConfig {
			version: Some(version),
			launch: LaunchConfig {
				java: Some(profile.java_dir),
				args,
				..Default::default()
			},
			game_dir: Some(profile.game_dir),
			..Default::default()
		},
		..Default::default()
	})
}

/// launcher_profiles.json
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherProfiles {
	profiles: HashMap<String, Profile>,
}

/// A single launcher profile
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
	name: String,
	#[serde(rename = "type")]
	kind: ProfileType,
	last_version_id: String,
	game_dir: String,
	java_dir: String,
	java_args: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProfileType {
	Custom,
	LatestRelease,
	LatestSnapshot,
}
