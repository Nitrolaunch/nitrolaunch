use std::{
	collections::{HashMap, HashSet},
	path::Path,
};

use anyhow::Context;
use clap::Parser;
use color_print::cprintln;
use nitro_core::{io::json_from_file, net::game_files::assets::AssetIndex};
use nitro_instance::{addon::storage::get_sha256_addon_path, lock::InstanceLockfile};
use nitro_plugin::api::executable::ExecutablePlugin;
use nitro_shared::{io::dir_size, java_args::MemoryNum};

fn main() -> anyhow::Result<()> {
	let mut plugin = ExecutablePlugin::from_manifest_file("cleanup", include_str!("plugin.json"))?;
	plugin.subcommand(|ctx, arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "cleanup" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(arg.args.into_iter().skip(1));
		let cli = Cli::parse_from(it);

		let data_dir = ctx.get_data_dir()?;

		let runtime = tokio::runtime::Runtime::new()?;
		runtime.block_on(async {
			match cli.subcommand {
				Subcommand::Version { version } => cleanup_version(&data_dir, &version).await,
				Subcommand::Addons => cleanup_addons(&data_dir).await,
				Subcommand::FabricCache => cleanup_fabric_cache(&data_dir).await,
			}
		})?;

		Ok(())
	})?;

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
	#[command(about = "Remove the assets for a Minecraft version")]
	Version {
		/// The Minecraft version
		version: String,
	},
	#[command(about = "Remove unused versions of addons for packages")]
	Addons,
	#[command(about = "Remove Fabric mod caches")]
	FabricCache,
}

async fn cleanup_version(data_dir: &Path, version: &str) -> anyhow::Result<()> {
	// First load all of the asset indexes
	let mut indexes = HashMap::new();
	for entry in data_dir
		.join("internal/assets/indexes")
		.read_dir()
		.context("Failed to read asset index directory")?
	{
		let entry = entry?;
		let path = entry.path();
		let Some(file_stem) = path.file_stem() else {
			continue;
		};

		let version = file_stem.to_string_lossy().to_string();

		let data: AssetIndex = json_from_file(&path)
			.with_context(|| format!("Failed to read asset index for version {version}"))?;

		indexes.insert(version, (data, path));
	}

	// Get the index for the version we want to remove
	let (version_index, version_index_path) = indexes
		.remove(version)
		.context("Version not found in asset indexes. Are you sure it is installed?")?;
	cprintln!("<s>Comparing assets...");
	let mut unique_assets = HashSet::with_capacity(version_index.objects.len());
	unique_assets.extend(version_index.objects.values().map(|x| x.hash.clone()));
	for (index, _) in indexes.values() {
		for object in index.objects.values() {
			unique_assets.remove(&object.hash);
		}
	}

	cprintln!("<s>Removing {} assets...", unique_assets.len());
	for hash in unique_assets {
		let subpath = format!("{}/{hash}", &hash[0..2]);
		let path = data_dir.join("internal/assets/objects").join(subpath);
		if path.exists() {
			let _ = std::fs::remove_file(path);
		}
	}

	// Remove the index so it doesn't affect other indexes anymore
	std::fs::remove_file(version_index_path).context("Failed to remove asset index")?;

	// Remove the game jar
	let jars_path = data_dir.join("internal/jars");
	let client_file_stub = format!("{version}_client");
	let server_file_stub = format!("{version}_server");
	for entry in jars_path.read_dir()? {
		let entry = entry?;
		if entry.file_type()?.is_file() {
			let filename = entry.file_name().to_string_lossy().to_string();
			if filename.contains(&client_file_stub) || filename.contains(&server_file_stub) {
				let _ = std::fs::remove_file(entry.path());
			}
		}
	}

	cprintln!("<s><g>Done.");

	Ok(())
}

async fn cleanup_addons(data_dir: &Path) -> anyhow::Result<()> {
	// First collect all hashed sources from lockfiles
	let mut lockfile_paths = Vec::new();

	let backup_lock_dir = data_dir.join("internal/lock/instances");
	if backup_lock_dir.exists() {
		for entry in backup_lock_dir.read_dir()? {
			let entry = entry?;
			lockfile_paths.push(entry.path());
		}
	}

	for entry in data_dir.join("instances").read_dir()? {
		let entry = entry?;

		for possible_dir in [".minecraft", "."] {
			let lockfile = entry.path().join(possible_dir).join("nitro_lock.json");
			if lockfile.exists() {
				lockfile_paths.push(lockfile);
			}
		}
	}

	let mut used_addons = HashSet::new();
	let addons_dir = data_dir.join("internal/addons");
	for path in lockfile_paths {
		let lockfile = InstanceLockfile::open(&path)?;
		for addon in lockfile.get_addons() {
			if let Some(hash) = &addon.hashes.sha256 {
				used_addons.insert(get_sha256_addon_path(&addons_dir, hash));
			}
		}
	}

	cprintln!("<s>Removing addons...");

	let mut removed_count = 0;
	let mut removed_size = 0;

	for addon_dir in ["sha256"] {
		let addon_dir = data_dir.join("internal/addons").join(addon_dir);

		for entry in addon_dir.read_dir()? {
			let entry = entry?;
			let path = entry.path();
			if !used_addons.contains(&path) {
				if let Ok(meta) = entry.metadata() {
					removed_size += meta.len() as usize;
				}
				removed_count += 1;

				let _ = tokio::fs::remove_file(path).await;
			}
		}
	}

	cprintln!("<s><g>Done.");
	cprintln!(
		"<s>Removed {removed_count} files totalling {}",
		MemoryNum::from_bytes(removed_size)
	);
	Ok(())
}

async fn cleanup_fabric_cache(data_dir: &Path) -> anyhow::Result<()> {
	cprintln!("<s>Removing cache...");
	let mut removed_size = 0;

	for entry in data_dir.join("instances").read_dir()? {
		let Ok(entry) = entry else {
			continue;
		};

		for possible_dir in [".minecraft", "."] {
			let possible_dir = entry.path().join(possible_dir);
			for cache_dir in ["processedMods", "remappedJars"] {
				let dir = possible_dir.join(".fabric").join(cache_dir);

				if dir.exists() {
					if let Ok(size) = dir_size(&dir) {
						removed_size += size;
					}
					let _ = tokio::fs::remove_dir_all(dir).await;
				}
			}
		}
	}

	cprintln!("<s><g>Done.");
	cprintln!(
		"<s>Removed files totalling {}",
		MemoryNum::from_bytes(removed_size)
	);

	Ok(())
}
