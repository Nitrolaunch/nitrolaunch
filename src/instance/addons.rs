use std::path::{Path, PathBuf};

use anyhow::{ensure, Context};
use nitro_config::instance::get_addon_paths;
use nitro_shared::addon::Addon;
use nitro_shared::versions::VersionInfo;

use crate::addon::{self, AddonExt};
use crate::io::paths::Paths;

use super::Instance;

impl Instance {
	/// Creates an addon on the instance
	pub fn create_addon(
		&mut self,
		addon: &Addon,
		selected_worlds: &[String],
		paths: &Paths,
		version_info: &VersionInfo,
	) -> anyhow::Result<()> {
		self.ensure_dirs(paths)?;

		for path in self
			.get_linked_addon_paths(addon, selected_worlds, paths, version_info)
			.context("Failed to get linked directory")?
		{
			Self::link_addon(&path, addon, paths, &self.id)
				.with_context(|| format!("Failed to link addon {}", addon.id))?;
		}

		Ok(())
	}

	/// Get the paths on this instance to hardlink an addon to
	pub fn get_linked_addon_paths(
		&mut self,
		addon: &Addon,
		selected_worlds: &[String],
		paths: &Paths,
		version_info: &VersionInfo,
	) -> anyhow::Result<Vec<PathBuf>> {
		self.ensure_dirs(paths)?;
		let game_dir = &self.dirs.get().game_dir;
		get_addon_paths(
			&self.config.original_config_with_profiles_and_plugins,
			game_dir,
			addon.kind,
			selected_worlds,
			version_info,
		)
	}

	/// Hardlinks the addon from the path in addon storage to the correct in the instance,
	/// under the specified directory
	fn link_addon(
		dir: &Path,
		addon: &Addon,
		paths: &Paths,
		instance_id: &str,
	) -> anyhow::Result<()> {
		let link = dir.join(addon.file_name.clone());
		let addon_path = addon.get_path(paths, instance_id);
		nitro_core::io::files::create_leading_dirs(&link)?;
		// These checks are to make sure that we properly link the hardlink to the right location
		// We have to remove the current link since it doesnt let us update it in place
		ensure!(addon_path.exists(), "Addon path does not exist");
		if link.exists() {
			std::fs::remove_file(&link).context("Failed to remove instance addon file")?;
		}
		nitro_core::io::files::update_hardlink(&addon_path, &link)
			.context("Failed to create hard link")?;
		Ok(())
	}

	/// Removes an addon file from this instance
	pub fn remove_addon_file(&self, path: &Path, paths: &Paths) -> anyhow::Result<()> {
		// We check if it is a stored addon path due to the old behavior to put that path in the lockfile.
		// Also some other sanity checks
		if path.exists() && !addon::is_stored_addon_path(path, paths) && !path.is_dir() {
			std::fs::remove_file(path).context("Failed to remove instance addon file")?;
		}

		Ok(())
	}
}
