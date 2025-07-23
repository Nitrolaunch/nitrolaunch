use std::{
	fs::File,
	io::BufReader,
	path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use nitrolaunch::config_crate::instance::{
	Args, CommonInstanceConfig, InstanceConfig, LaunchArgs, LaunchConfig, LaunchMemory, QuickPlay,
};
use nitro_core::io::java::install::JavaInstallationKind;
use nitro_plugin::{api::CustomPlugin, hooks::ImportInstanceResult};
use nitro_shared::{
	loaders::Loader,
	output::{NitroOutput, MessageContents, MessageLevel},
	versions::parse_versioned_string,
	Side,
};
use serde::{Deserialize, Serialize};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

fn main() -> anyhow::Result<()> {
	let mut plugin =
		CustomPlugin::from_manifest_file("xmcl_transfer", include_str!("plugin.json"))?;

	plugin.export_instance(|_, arg| {
		if arg.config.side == Some(Side::Server) {
			bail!("Servers cannot be exported to XMCL");
		}

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
		zip.start_file("instance.json", FileOptions::<()>::default())
			.context("Failed to create metadata file in export")?;

		let (min_mem, max_mem) = arg.config.common.launch.memory.to_min_max();

		fn get_loader_version(
			actual_loader: Option<&Loader>,
			checked_loader: Loader,
			loader_version: &Option<String>,
		) -> String {
			if actual_loader == Some(&checked_loader) {
				loader_version.clone().unwrap_or_default()
			} else {
				String::new()
			}
		}

		let java = if let JavaInstallationKind::Custom { .. } =
			JavaInstallationKind::parse(&arg.config.common.launch.java)
		{
			arg.config.common.launch.java.clone()
		} else {
			String::new()
		};

		let server =
			if let QuickPlay::Server { server, port } = &arg.config.common.launch.quick_play {
				Server {
					host: server.clone(),
					port: port.unwrap_or(25565),
				}
			} else {
				Server {
					host: String::new(),
					port: 0,
				}
			};

		let loader = arg
			.config
			.common
			.loader
			.as_ref()
			.map(|x| Loader::parse_from_str(parse_versioned_string(x).0));

		let meta = Metadata {
			name: arg.config.name.unwrap_or_default(),
			min_memory: min_mem.unwrap_or_default().to_bytes(),
			max_memory: max_mem.unwrap_or_default().to_bytes(),
			vm_options: arg.config.common.launch.args.jvm.parse(),
			mc_options: arg.config.common.launch.args.game.parse(),
			runtime: RuntimeMetadata {
				minecraft: arg.minecraft_version,
				forge: get_loader_version(loader.as_ref(), Loader::Forge, &arg.loader_version),
				liteloader: get_loader_version(
					loader.as_ref(),
					Loader::LiteLoader,
					&arg.loader_version,
				),
				fabric_loader: get_loader_version(
					loader.as_ref(),
					Loader::Fabric,
					&arg.loader_version,
				),
				yarn: String::new(),
				optifine: String::new(),
				quilt_loader: get_loader_version(
					loader.as_ref(),
					Loader::Quilt,
					&arg.loader_version,
				),
			},
			java,
			version: String::new(),
			server,
		};

		serde_json::to_writer(&mut zip, &meta).context("Failed to write metadata file")?;

		Ok(())
	})?;

	plugin.import_instance(|mut ctx, arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		// Read the metadata
		let mut zip = ZipArchive::new(File::open(source_path).context("Failed to open instance")?)?;
		let mut meta_file = zip
			.by_name("instance.json")
			.context("Metadata file is missing in instance")?;
		let meta: Metadata = serde_json::from_reader(&mut meta_file)
			.context("Failed to deserialize instance metadata")?;

		std::mem::drop(meta_file);

		// We need to write in the .minecraft directory for clients
		let target_path = target_path.join(".minecraft");

		// Extract all the instance files
		zip.extract(target_path)
			.context("Failed to extract instance")?;

		let (loader, loader_version) = if !meta.runtime.forge.is_empty() {
			(Loader::Forge, Some(meta.runtime.forge))
		} else if !meta.runtime.liteloader.is_empty() {
			(Loader::LiteLoader, Some(meta.runtime.liteloader))
		} else if !meta.runtime.fabric_loader.is_empty() {
			(Loader::Fabric, Some(meta.runtime.fabric_loader))
		} else if !meta.runtime.quilt_loader.is_empty() {
			(Loader::Quilt, Some(meta.runtime.quilt_loader))
		} else if !meta.runtime.optifine.is_empty() || !meta.runtime.yarn.is_empty() {
			ctx.get_output().display(
				MessageContents::Warning(
					"Nitrolaunch does not understand the instance's loader".into(),
				),
				MessageLevel::Important,
			);
			(Loader::Vanilla, None)
		} else {
			(Loader::Vanilla, None)
		};

		let quick_play = if !meta.server.host.is_empty() {
			QuickPlay::Server {
				server: meta.server.host,
				port: Some(meta.server.port),
			}
		} else {
			QuickPlay::None
		};

		let loader = serde_json::to_string(&loader)?.replace("\"", "");
		let loader = if let Some(loader_version) = loader_version {
			format!("{loader}@{loader_version}")
		} else {
			loader
		};

		Ok(ImportInstanceResult {
			format: arg.format,
			config: InstanceConfig {
				name: Some(meta.name),
				common: CommonInstanceConfig {
					loader: Some(loader),
					launch: LaunchConfig {
						memory: LaunchMemory::Both {
							min: meta.min_memory.to_string(),
							max: meta.max_memory.to_string(),
						},
						args: LaunchArgs {
							jvm: Args::List(meta.vm_options),
							game: Args::List(meta.mc_options),
						},
						java: if meta.java.is_empty() {
							"auto".into()
						} else {
							meta.java
						},
						quick_play,
						..Default::default()
					},
					..Default::default()
				},
				..Default::default()
			},
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

/// Metadata file for XMCL instances
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
	name: String,
	min_memory: u64,
	max_memory: u64,
	vm_options: Vec<String>,
	mc_options: Vec<String>,
	runtime: RuntimeMetadata,
	/// Path to Java, empty means auto
	java: String,
	version: String,
	server: Server,
}

/// Runtime metadata (versions of everything)
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeMetadata {
	minecraft: String,
	forge: String,
	liteloader: String,
	fabric_loader: String,
	yarn: String,
	optifine: String,
	quilt_loader: String,
}

/// Autolaunch server
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Server {
	host: String,
	port: u16,
}
