use anyhow::{bail, Context};
use nitro_plugin::hook::hooks::{DeleteInstance, SaveInstanceConfigArg};
use nitro_shared::{
	id::InstanceID, io::dir_size, output::NitroOutput, util::DeserListOrSingle, Side,
};

use crate::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	instance::Instance,
	io::paths::Paths,
	plugin::PluginManager,
};

impl Instance {
	/// Consolidates the parent configs of this instance into it's config, and saves the result
	pub async fn consolidate(
		&self,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let mut config = self.config.clone();
		config.from = DeserListOrSingle::default();

		let modifications = vec![ConfigModification::AddInstance(self.id.clone(), config)];
		let mut config = Config::open(&Config::get_path(paths))?;

		apply_modifications_and_write(&mut config, modifications, paths, plugins, o).await
	}

	/// Duplicates this instance to create a new one
	pub async fn duplicate(
		&self,
		new_id: &InstanceID,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let config = self.original_config.clone();

		let modifications = vec![ConfigModification::AddInstance(new_id.clone(), config)];
		let mut config = Config::open(&Config::get_path(paths))?;

		apply_modifications_and_write(&mut config, modifications, paths, plugins, o).await
	}

	/// Deletes this instance and all of its files. Use with caution!
	pub async fn delete(
		&self,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		self.delete_files()
			.await
			.context("Failed to delete files")?;

		if let Some(source_plugin) = &self.original_config.source_plugin {
			if !self.original_config.is_deletable {
				bail!("Plugin instance does not support deletion");
			}

			let arg = SaveInstanceConfigArg {
				id: self.id.to_string(),
				config: self.original_config.clone(),
			};

			let result = plugins
				.call_hook_on_plugin(DeleteInstance, source_plugin, &arg, paths, o)
				.await?;
			if let Some(result) = result {
				result.result(o).await?;
			}
		} else {
			let mut config = Config::open(&Config::get_path(paths))?;
			let modifications = vec![ConfigModification::RemoveInstance(self.id.clone())];
			apply_modifications_and_write(&mut config, modifications, paths, plugins, o)
				.await
				.context("Failed to modify and write config")?;
		}

		Ok(())
	}

	/// Removes all game files for an instance, including saves. Does not remove the instance from config. Use with caution!
	pub async fn delete_files(&self) -> anyhow::Result<()> {
		if let Some(dir) = &self.dir {
			// Remove the parent directory above .minecraft for clients
			let path = if self.config.dir.is_none() && self.side() == Side::Client {
				if let Some(parent) = dir.parent() {
					if parent
						.file_name()
						.is_some_and(|x| x.to_string_lossy() == "instances")
					{
						bail!("Attempted to remove instances directory");
					}
					parent
				} else {
					dir
				}
			} else {
				dir
			};

			tokio::fs::remove_dir_all(path).await?;
		}

		Ok(())
	}

	/// Gets the size of this instance on the disk
	pub async fn get_size(&self) -> anyhow::Result<usize> {
		let Some(dir) = &self.dir else {
			return Ok(0);
		};

		if !dir.exists() {
			return Ok(0);
		}

		dir_size(dir)
	}
}
