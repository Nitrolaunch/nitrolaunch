use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_shared::{
	minecraft::AddonKind,
	versions::{VersionInfo, VersionPattern},
	Side,
};

/// Modpack formats
pub mod modpack;
/// Addon storage
pub mod storage;

/// Some content that is installed on Minecraft
#[derive(Debug, Clone)]
pub struct Addon {
	/// What type of addon this is
	pub kind: AddonKind,
	/// The addon's file name
	pub file_name: String,
	/// The original path where the addon was read from. May not be all the paths where the final addon is linked.
	pub path: Option<PathBuf>,
	/// The source / link-stored file for the addon
	pub source: Option<PathBuf>,
}

impl Addon {
	/// Reads an addon from the filesystem
	pub fn from_file(path: &Path, kind: AddonKind) -> anyhow::Result<Self> {
		Ok(Self {
			kind,
			file_name: path
				.file_name()
				.context("Addon is not a file")?
				.to_string_lossy()
				.to_string(),
			path: Some(path.to_owned()),
			source: None,
		})
	}

	/// Gets the target paths for this addon on an instance
	pub fn get_targets(
		&self,
		side: Side,
		inst_dir: &Path,
		selected_worlds: &[String],
		datapack_folder: Option<&Path>,
		version_info: &VersionInfo,
	) -> Vec<PathBuf> {
		get_addon_dirs(
			self.kind,
			side,
			inst_dir,
			selected_worlds,
			datapack_folder,
			version_info,
		)
		.into_iter()
		.map(|x| x.join(&self.file_name))
		.collect()
	}

	/// Links this addon's targets to its source
	pub fn link(&self, targets: &[PathBuf]) -> std::io::Result<()> {
		let Some(source) = &self.source else {
			return Ok(());
		};
		let mut result = Ok(());

		for target in targets {
			if let Some(parent) = target.parent() {
				let _ = std::fs::create_dir_all(parent);
			}
			if target.exists() {
				let _ = std::fs::remove_file(target);
			}
			#[allow(deprecated)]
			let single_result = std::fs::soft_link(source, target);
			if result.is_ok() {
				result = single_result;
			}
		}

		result
	}
}

/// Get the directories on an instance where addons are stored
pub fn get_addon_dirs(
	addon: AddonKind,
	side: Side,
	inst_dir: &Path,
	selected_worlds: &[String],
	datapack_folder: Option<&Path>,
	version_info: &VersionInfo,
) -> Vec<PathBuf> {
	match addon {
		AddonKind::ResourcePack => match side {
			Side::Client => vec![get_resource_pack_dir(
				inst_dir,
				side,
				VersionPattern::After("13w24a".into()).matches_info(version_info),
			)],
			// No resource packs are actually loaded from here, but a plugin could take advantage
			Side::Server => vec![inst_dir.join("resourcepacks")],
		},
		AddonKind::Mod => vec![inst_dir.join("mods")],
		AddonKind::Plugin => match side {
			Side::Client => vec![],
			Side::Server => vec![inst_dir.join("plugins")],
		},
		AddonKind::Shader => match side {
			Side::Client => vec![inst_dir.join("shaderpacks")],
			Side::Server => vec![],
		},
		AddonKind::Datapack => get_datapack_dirs(side, inst_dir, selected_worlds, datapack_folder),
	}
}

/// Get directory to store resource packs in an instance
pub fn get_resource_pack_dir(inst_dir: &Path, side: Side, after_13w24a: bool) -> PathBuf {
	match side {
		Side::Client => {
			// Resource packs are texture packs on older versions
			if after_13w24a {
				inst_dir.join("resourcepacks")
			} else {
				inst_dir.join("texturepacks")
			}
		}
		// No resource packs are actually loaded from here, but a plugin could take advantage
		Side::Server => inst_dir.join("resourcepacks"),
	}
}

/// Get directories to store datapacks in on an instance
pub fn get_datapack_dirs(
	side: Side,
	inst_dir: &Path,
	selected_worlds: &[String],
	datapack_folder: Option<&Path>,
) -> Vec<PathBuf> {
	if let Some(datapack_folder) = datapack_folder {
		vec![inst_dir.join(datapack_folder)]
	} else {
		match side {
			Side::Client => {
				if selected_worlds.is_empty() {
					vec![inst_dir.join("world_files/datapacks")]
				} else {
					selected_worlds
						.iter()
						.map(|x| inst_dir.join("saves").join(x).join("datapacks"))
						.collect()
				}
			}
			Side::Server => {
				// TODO: Support custom world names
				vec![inst_dir.join("world").join("datapacks")]
			}
		}
	}
}

/// Checks for a valid addon filename
pub fn is_filename_valid(kind: AddonKind, filename: &str) -> bool {
	filename.ends_with(kind.get_extension())
}
