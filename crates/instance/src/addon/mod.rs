use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_shared::{
	Side,
	io::update_link,
	minecraft::AddonKind,
	pkg::AddonOptionalHashes,
	versions::{VersionInfo, VersionPattern},
};
use serde::{Deserialize, Serialize};

/// Modpack formats
pub mod modpack;
/// Addon storage
pub mod storage;

/// Some content that is installed on Minecraft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Addon {
	/// What type of addon this is
	pub kind: AddonKind,
	/// The addon's file name
	pub file_name: String,
	/// The original path where the addon was read from. May not be all the paths where the final addon is linked.
	pub original_path: Option<PathBuf>,
	/// Target paths for the final addon to be linked to on an instance
	pub target_paths: Vec<PathBuf>,
	/// The source / link-stored file for the addon
	pub source: Option<PathBuf>,
	/// Hashes for the addon
	pub hashes: AddonOptionalHashes,
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
			original_path: Some(path.to_owned()),
			target_paths: Vec::new(),
			source: None,
			hashes: AddonOptionalHashes::default(),
		})
	}

	/// Gets the target paths for this addon on an instance
	pub fn get_targets(
		&mut self,
		side: Side,
		inst_dir: &Path,
		selected_worlds: &[String],
		datapack_folder: Option<&Path>,
		version_info: &VersionInfo,
	) {
		self.target_paths = get_addon_dirs(
			self.kind,
			side,
			inst_dir,
			selected_worlds,
			datapack_folder,
			version_info,
		)
		.into_iter()
		.map(|x| x.join(&self.file_name))
		.collect();
	}

	/// Links this addon's targets to its source
	pub fn link(&self) -> std::io::Result<()> {
		let Some(source) = &self.source else {
			return Ok(());
		};
		let mut result = Ok(());

		for target in &self.target_paths {
			if let Some(parent) = target.parent() {
				let _ = std::fs::create_dir_all(parent);
			}
			if target.exists() {
				let _ = std::fs::remove_file(target);
			}
			let single_result = update_link(source, target);
			if result.is_ok() {
				result = single_result;
			}
		}

		result
	}

	/// Removes all instances of this addon from the instance
	pub fn remove_from_instance(&self) -> anyhow::Result<()> {
		for target in &self.target_paths {
			if target.exists() {
				std::fs::remove_file(target)?;
			}
		}

		Ok(())
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
		AddonKind::Modpack => vec![],
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
