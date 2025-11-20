#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
use std::{
	collections::{HashMap, HashSet},
	path::Path,
};

use anyhow::Context;
use clap::Parser;
use color_print::cprintln;
use nitro_core::{io::json_from_file, net::game_files::assets::AssetIndex};
use nitro_plugin::api::executable::ExecutablePlugin;

fn main() -> anyhow::Result<()> {
	let mut plugin = ExecutablePlugin::from_manifest_file("cleanup", include_str!("plugin.json"))?;
	plugin.subcommand(|ctx, args| {
		let Some(subcommand) = args.first() else {
			return Ok(());
		};
		if subcommand != "cleanup" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(args.into_iter().skip(1));
		let cli = Cli::parse_from(it);

		let data_dir = ctx.get_data_dir()?;

		let runtime = tokio::runtime::Runtime::new()?;
		runtime.block_on(async {
			match cli.subcommand {
				Subcommand::Version { version } => cleanup_version(&data_dir, &version).await,
				Subcommand::Addons => cleanup_addons(&data_dir).await,
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
	let mut removed_count = 0;
	let mut removed_size = 0;

	fn walk_function(
		dir: &Path,
		removed_count: &mut usize,
		removed_size: &mut usize,
	) -> anyhow::Result<()> {
		let read = dir.read_dir()?;
		for entry in read {
			let Ok(entry) = entry else {
				continue;
			};

			if entry.file_type()?.is_dir() {
				walk_function(&entry.path(), removed_count, removed_size)?;
			} else {
				let Ok(meta) = std::fs::metadata(entry.path()) else {
					continue;
				};

				let mut should_remove = false;

				#[cfg(target_family = "unix")]
				{
					// If the file only has one link then it is unused
					if meta.nlink() == 1 {
						should_remove = true;
					}
				}
				#[cfg(not(target_family = "unix"))]
				{
					should_remove = true;
				}
				if should_remove {
					let _ = tokio::spawn(tokio::fs::remove_file(entry.path()));
					*removed_count += 1;
					*removed_size += meta.len() as usize;
				}
			}
		}

		Ok(())
	}

	cprintln!("<s>Removing addons...");

	walk_function(
		&data_dir.join("internal/addons"),
		&mut removed_count,
		&mut removed_size,
	)?;

	cprintln!("<s><g>Done.");
	cprintln!(
		"<s>Removed {removed_count} files totalling {}MB",
		removed_size / 1024 / 1024
	);
	Ok(())
}
