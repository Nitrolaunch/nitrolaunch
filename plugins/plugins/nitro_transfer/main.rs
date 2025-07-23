use std::{
	fs::File,
	io::BufReader,
	path::{Path, PathBuf},
};

use anyhow::Context;
use nitrolaunch::config_crate::instance::InstanceConfig;
use nitro_plugin::{api::CustomPlugin, hooks::ImportInstanceResult};
use nitro_shared::Side;
use serde::{Deserialize, Serialize};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

fn main() -> anyhow::Result<()> {
	let mut plugin =
		CustomPlugin::from_manifest_file("nitro_transfer", include_str!("plugin.json"))?;

	plugin.export_instance(|_, arg| {
		let game_dir = PathBuf::from(arg.game_dir);
		let target_path = PathBuf::from(arg.result_path);
		let target_file = File::create(target_path).context("Failed to open target file")?;

		// Write the instance files
		let mut zip = ZipWriter::new(target_file);

		visit_dir(&game_dir, &mut zip, &game_dir).context("Failed to read instance directory")?;

		fn visit_dir(dir: &Path, zip: &mut ZipWriter<File>, game_dir: &Path) -> anyhow::Result<()> {
			let dir_read = dir.read_dir().context("Failed to read directory")?;

			for item in dir_read {
				let item = item?;
				if item.file_type()?.is_dir() {
					visit_dir(&item.path(), zip, game_dir)?;
				} else {
					if !should_include_file(&item.path()) {
						continue;
					}

					zip.start_file_from_path(
						item.path().strip_prefix(game_dir)?,
						FileOptions::<()>::default(),
					)?;
					let mut src = BufReader::new(File::open(item.path())?);
					std::io::copy(&mut src, zip).context("Failed to copy file into ZIP")?;
				}
			}

			Ok(())
		}

		// Write the metadata file
		zip.start_file("nitro_meta.json", FileOptions::<()>::default())
			.context("Failed to create metadata file in export")?;

		let meta = Metadata {
			id: arg.id,
			config: arg.config,
		};

		serde_json::to_writer(&mut zip, &meta).context("Failed to write metadata file")?;

		Ok(())
	})?;

	plugin.import_instance(|_, arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		// Read the metadata
		let mut zip = ZipArchive::new(File::open(source_path).context("Failed to open instance")?)?;
		let mut meta_file = zip
			.by_name("nitro_meta.json")
			.context("Metadata file is missing in instance")?;
		let meta: Metadata = serde_json::from_reader(&mut meta_file)
			.context("Failed to deserialize instance metadata")?;

		std::mem::drop(meta_file);

		// We need to write in the .minecraft directory for clients
		let target_path = match meta.config.side.context("Side is missing in metadata")? {
			Side::Client => target_path.join(".minecraft"),
			Side::Server => target_path,
		};

		// Extract all the instance files
		zip.extract(target_path)
			.context("Failed to extract instance")?;

		Ok(ImportInstanceResult {
			format: arg.format,
			config: meta.config,
		})
	})?;

	Ok(())
}

/// Checks if a file should be included in the export
fn should_include_file(path: &Path) -> bool {
	if let Some(file_name) = path.file_name() {
		let file_name = file_name.to_string_lossy();
		if file_name.starts_with("nitro_") {
			return false;
		}
	}

	true
}

/// Metadata file for exported instances
#[derive(Serialize, Deserialize)]
struct Metadata {
	id: String,
	config: InstanceConfig,
}
