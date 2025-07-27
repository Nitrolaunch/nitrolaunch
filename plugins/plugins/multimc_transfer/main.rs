use std::{collections::HashMap, fs::File, path::PathBuf};

use anyhow::Context;
use nitro_core::{io::extract_zip_dir, util::versions::MinecraftVersionDeser};
use nitro_plugin::{api::CustomPlugin, hooks::ImportInstanceResult};
use nitro_shared::{loaders::Loader, Side};
use nitrolaunch::config_crate::instance::{CommonInstanceConfig, InstanceConfig};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

fn main() -> anyhow::Result<()> {
	let mut plugin =
		CustomPlugin::from_manifest_file("multimc_transfer", include_str!("plugin.json"))?;

	plugin.import_instance(|_, arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		let mut zip = ZipArchive::new(File::open(source_path).context("Failed to open instance")?)?;

		// Read the CFG file
		let mut cfg_file = zip
			.by_name("instance.cfg")
			.context("CFG file is missing in instance")?;
		let cfg =
			std::io::read_to_string(&mut cfg_file).context("Failed to read instance config")?;
		let mut cfg = read_instance_cfg(&cfg);
		std::mem::drop(cfg_file);

		let name = cfg.remove("name");

		let mut mmc_pack_file = zip
			.by_name("mmc-pack.json")
			.context("MMC Pack file is missing in instance")?;
		let mmc_pack: MMCPack = serde_json::from_reader(&mut mmc_pack_file)
			.context("Failed to deserialize instance MMC pack")?;

		std::mem::drop(mmc_pack_file);

		// Figure out loader and Minecraft version
		let mut version = None;
		let mut loader = Loader::Vanilla;

		for component in mmc_pack.components {
			if component.uid == "net.minecraft" {
				version = Some(component.version);
			}
			if component.uid == "net.fabricmc.fabric-loader" {
				loader = Loader::Fabric;
			}
		}
		let version = version.context("No Minecraft version provided")?;

		// Write the instance files

		// We need to write in the .minecraft directory for clients
		let target_path = target_path.join(".minecraft");

		// Extract all the instance files
		extract_zip_dir(&mut zip, "minecraft", target_path)
			.context("Failed to extract instance files")?;

		Ok(ImportInstanceResult {
			format: arg.format,
			config: InstanceConfig {
				name: name.map(|x| x.to_string()),
				side: Some(Side::Client),
				common: CommonInstanceConfig {
					version: Some(MinecraftVersionDeser::Version(version.into())),
					loader: Some(loader.to_string().to_lowercase()),
					..Default::default()
				},
				..Default::default()
			},
		})
	})?;

	Ok(())
}

/// mmc-pack.json format
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MMCPack {
	components: Vec<MMCComponent>,
}

/// Single component in mmc-pack.json
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MMCComponent {
	cached_name: String,
	version: String,
	uid: String,
}

/// Reads the instance.cfg file to a set of fields
fn read_instance_cfg(contents: &str) -> HashMap<&str, &str> {
	let mut out = HashMap::new();

	for line in contents.lines() {
		let Some((key, value)) = line.split_once("=") else {
			continue;
		};

		out.insert(key, value);
	}

	out
}
