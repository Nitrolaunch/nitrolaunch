use std::{
	collections::HashMap,
	fs::File,
	path::{Path, PathBuf},
};

use anyhow::Context;
use nitro_core::{io::extract_zip_dir, util::versions::MinecraftVersionDeser};
use nitro_plugin::{api::CustomPlugin, hooks::ImportInstanceResult};
use nitro_shared::{loaders::Loader, Side};
use nitrolaunch::config_crate::{
	instance::{CommonInstanceConfig, InstanceConfig},
	package::PackageConfigDeser,
};
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
		extract_zip_dir(&mut zip, "minecraft", &target_path)
			.context("Failed to extract instance files")?;

		// Replace addons with packages
		let mut all_packages = Vec::new();

		for addon_dir in [
			"mods",
			"resourcepacks",
			"texturepacks",
			"shaderpacks",
			"shaders",
		] {
			all_packages.extend(addons_to_packages(&target_path.join(addon_dir))?);
		}

		Ok(ImportInstanceResult {
			format: arg.format,
			config: InstanceConfig {
				name: name.map(|x| x.to_string()),
				side: Some(Side::Client),
				common: CommonInstanceConfig {
					version: Some(MinecraftVersionDeser::Version(version.into())),
					loader: Some(loader.to_string().to_lowercase()),
					packages: all_packages
						.into_iter()
						.map(|x| PackageConfigDeser::Basic(x.into()))
						.collect(),
					..Default::default()
				},
				..Default::default()
			},
		})
	})?;

	Ok(())
}

/// Converts addons to packages in the given addon directory (resourcepacks, mods, etc.)
fn addons_to_packages(dir: &Path) -> anyhow::Result<Vec<String>> {
	if !dir.exists() {
		return Ok(Vec::new());
	}

	// The .index dir in the addon dir will have a list of TOML files with info about each addon
	let index = dir.join(".index");
	if index.exists() {
		let mut out = Vec::new();
		for index_file in index.read_dir().context("Failed to read index directory")? {
			let index_file = index_file?;
			let contents = std::fs::read_to_string(index_file.path())
				.context("Failed to read index file contents")?;
			let toml = read_pw_toml(&contents);

			let global_section = toml.get("global").context("Global section missing")?;
			let filename = global_section.get("filename").context("Filename missing")?;

			// Whether we are replacing this addon with a package
			let mut is_packaged = false;

			if let Some(modrinth) = toml.get("update.modrinth") {
				let id = modrinth.get("mod-id").context("ID missing")?;

				is_packaged = true;
				out.push(format!("modrinth:{id}"));
			}

			if is_packaged {
				let path = dir.join(filename);
				if path.exists() && path.is_file() {
					std::fs::remove_file(path).context("Failed to remove addon")?;
				}
			}
		}

		Ok(out)
	} else {
		Ok(Vec::new())
	}
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

/// Reads the .pw.toml file for an addon
fn read_pw_toml(contents: &str) -> HashMap<&str, HashMap<&str, &str>> {
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
