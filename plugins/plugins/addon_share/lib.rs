use std::{fs::File, io::BufReader, path::PathBuf, str::FromStr};

use anyhow::{bail, Context};
use clap::Parser;
use nitro_config::instance::get_addon_paths;
use nitro_plugin::{
	api::wasm::{
		sys::{get_current_dir, get_data_dir},
		WASMPlugin,
	},
	nitro_wasm_plugin,
};
use nitro_shared::{addon::AddonKind, id::InstanceID, versions::VersionInfo, Side};
use zip::{write::SimpleFileOptions, ZipWriter};

nitro_wasm_plugin!(main, "addon_share");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.subcommand(|arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "share-addons" {
			return Ok(());
		}

		// Trick the parser to give it the right bin name
		let it = std::iter::once("nitro instance share-addons".into())
			.chain(arg.args.into_iter().skip(1));
		let cli = Cli::try_parse_from(it)?;

		if cli.addons.is_empty() {
			bail!("No addon types specified");
		}

		let instance = arg
			.instances
			.get(&InstanceID::from(cli.instance.clone()))
			.context("Instance does not exist")?;
		let game_dir = if let Some(game_dir) = &instance.game_dir {
			PathBuf::from(game_dir)
		} else {
			let data_dir = get_data_dir();
			match instance.side.expect("Instance should have a side") {
				Side::Client => data_dir
					.join("instances")
					.join(&cli.instance)
					.join(".minecraft"),
				Side::Server => data_dir.join("instances").join(&cli.instance),
			}
		};

		// Collect the necessary addon dirs
		let mut dirs = Vec::new();
		for addon_type in &cli.addons {
			let kind = addon_type.to_addon_kind();
			// For resource packs we have to check both resourcepacks and texturepacks
			if *addon_type == AddonType::ResourcePacks {
				let version_info = VersionInfo {
					version: "foo".into(),
					versions: vec!["foo".into(), "13w24a".into()],
				};
				dirs.extend(get_addon_paths(
					instance,
					&game_dir,
					kind,
					&[],
					&version_info,
				)?);
				let version_info = VersionInfo {
					version: "foo".into(),
					versions: vec!["13w24a".into(), "foo".into()],
				};
				dirs.extend(get_addon_paths(
					instance,
					&game_dir,
					kind,
					&[],
					&version_info,
				)?);
			} else {
				let version_info = VersionInfo {
					version: "foo".into(),
					versions: Vec::new(),
				};
				dirs.extend(get_addon_paths(
					instance,
					&game_dir,
					kind,
					&[],
					&version_info,
				)?);
			}
		}

		// Create the zip file
		let output_filename = cli.output.unwrap_or_else(|| "addons.zip".into());
		// We have to canonicalize
		let output_path = get_current_dir().join(output_filename);
		let mut zip = ZipWriter::new(File::create(output_path).context("Failed to open zip file")?);
		for dir in dirs {
			if !dir.exists() {
				continue;
			}

			let read = dir.read_dir().context("Failed to read directory")?;
			for entry in read {
				let Ok(entry) = entry else {
					eprintln!("Failed to read addon");
					continue;
				};

				if !entry
					.file_type()
					.context("Failed to get file type")?
					.is_file()
				{
					continue;
				}

				let path = entry.path();

				// If there are multiple addon types, store them in separate subdirectories
				let target_path = if cli.addons.len() > 1 {
					let parent_name = path
						.parent()
						.expect("Should have a parent")
						.file_name()
						.expect("Should not be ..");

					PathBuf::from(parent_name).join(path.file_name().expect("Should be a file"))
				} else {
					PathBuf::from(entry.path().file_name().expect("Should be a file"))
				};

				let target_path = target_path.to_string_lossy().to_string();

				zip.start_file(target_path, SimpleFileOptions::default())
					.context("Failed to start zip file")?;

				std::io::copy(&mut BufReader::new(File::open(path)?), &mut zip)
					.context("Failed to copy file to zip")?;
			}
		}

		println!("Addons zipped!");

		Ok(())
	})?;

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {
	/// The instance to zip addons from
	instance: String,
	/// The types of addons to zip
	addons: Vec<AddonType>,
	/// The output filename
	#[arg(short, long)]
	output: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AddonType {
	Mods,
	ResourcePacks,
	Plugins,
	Shaders,
}

impl FromStr for AddonType {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"mods" | "mod" => Ok(Self::Mods),
			"resource_packs" | "resourcepacks" | "resource_pack" | "resourcepack" => {
				Ok(Self::ResourcePacks)
			}
			"plugins" | "plugin" => Ok(Self::Plugins),
			"shaders" | "shader" | "shaderpacks" | "shaderpack" | "shader_packs"
			| "shader_pack" => Ok(Self::Shaders),
			"datapacks" | "datapack" => Err("Datapacks are not supported by this plugin"),
			_ => Err("Unknown addon type"),
		}
	}
}

impl AddonType {
	fn to_addon_kind(self) -> AddonKind {
		match self {
			Self::Mods => AddonKind::Mod,
			Self::ResourcePacks => AddonKind::ResourcePack,
			Self::Plugins => AddonKind::Plugin,
			Self::Shaders => AddonKind::Shader,
		}
	}
}
