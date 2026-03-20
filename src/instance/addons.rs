use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_instance::addon::Addon;
use nitro_shared::versions::VersionInfo;

use crate::addon;
use crate::io::paths::Paths;

use super::Instance;

impl Instance {
	/// Creates or updates an addon on the instance
	pub fn create_addon(
		&mut self,
		addon: &Addon,
		selected_worlds: &[String],
		version_info: &VersionInfo,
	) -> anyhow::Result<()> {
		let targets = self.get_addon_targets(addon, selected_worlds, version_info);
		addon.link(&targets).context("Failed to link addon")
	}

	/// Get the target paths for an addon on this instance
	pub fn get_addon_targets(
		&mut self,
		addon: &Addon,
		selected_worlds: &[String],
		version_info: &VersionInfo,
	) -> Vec<PathBuf> {
		if let Some(inst_dir) = &self.dir {
			let config = &self.config.original_config_with_templates_and_plugins;
			addon.get_targets(
				self.get_side(),
				inst_dir,
				selected_worlds,
				config.datapack_folder.as_ref().map(|x| Path::new(x)),
				version_info,
			)
		} else {
			Vec::new()
		}
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
