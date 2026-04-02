use std::path::Path;

use anyhow::Context;
use nitro_instance::addon::Addon;
use nitro_shared::versions::VersionInfo;

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
			let config = &self.config.original_config_with_templates_and_plugins;
			addon.get_targets(
				self.get_side(),
				inst_dir,
				selected_worlds,
				config.datapack_folder.as_ref().map(|x| Path::new(x)),
				version_info,
			);
		}
	}
}
