use std::path::Path;

use anyhow::Context;
use nitro_instance::addon::{Addon, get_addon_dirs, get_resource_pack_dir};
use nitro_shared::{minecraft::AddonKind, versions::VersionInfo};

use super::Instance;

impl Instance {
	/// Creates or updates an addon on the instance
	pub fn create_addon(
		&mut self,
		addon: &Addon,
		selected_worlds: &[String],
		version_info: &VersionInfo,
	) -> anyhow::Result<()> {
		let mut addon = addon.clone();
		self.get_addon_targets(&mut addon, selected_worlds, version_info);
		addon.link().context("Failed to link addon")
	}

	/// Sets the target paths for an addon on this instance
	pub fn get_addon_targets(
		&mut self,
		addon: &mut Addon,
		selected_worlds: &[String],
		version_info: &VersionInfo,
	) {
		if let Some(inst_dir) = &self.dir {
			addon.get_targets(
				self.side(),
				inst_dir,
				selected_worlds,
				self.config.datapack_folder.as_ref().map(Path::new),
				version_info,
			);
		}
	}

	/// Gets all of the addons on this instance
	pub fn get_addons(&self) -> anyhow::Result<Vec<Addon>> {
		let Some(dir) = self.dir() else {
			return Ok(Vec::new());
		};

		let kinds = [
			AddonKind::Mod,
			AddonKind::Datapack,
			AddonKind::Plugin,
			AddonKind::ResourcePack,
			AddonKind::Shader,
		];

		let mut dirs = Vec::new();
		for kind in kinds {
			// For resource packs we have to check both resourcepacks and texturepacks
			if kind == AddonKind::ResourcePack {
				dirs.push((get_resource_pack_dir(dir, self.side(), false), kind));
				dirs.push((get_resource_pack_dir(dir, self.side(), true), kind));
			} else {
				let version_info = VersionInfo {
					version: "foo".into(),
					versions: Vec::new(),
				};
				let new_dirs = get_addon_dirs(
					kind,
					self.side(),
					dir,
					&[],
					self.config.datapack_folder.as_ref().map(Path::new),
					&version_info,
				);
				let new_dirs = new_dirs.into_iter().map(|x| (x, kind));
				dirs.extend(new_dirs);
			}
		}

		let mut addons = Vec::new();
		for (dir, kind) in dirs {
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

				addons.push(Addon::from_file(&entry.path(), kind)?);
			}
		}

		Ok(addons)
	}
}
