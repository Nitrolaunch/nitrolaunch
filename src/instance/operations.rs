use std::{
	io::{SeekFrom, Write},
	path::PathBuf,
	time::Duration,
};

use anyhow::{Context, bail};
use nitro_config::{
	instance::{InstanceConfig, LaunchMode},
	template::TemplateConfig,
};
use nitro_core::io::files::open_file_append;
use nitro_instance::files::{
	InstanceSave, ServerInfo, get_instance_saves, get_instance_screenshots, get_instance_servers,
};
use nitro_plugin::hook::hooks::{
	DeleteInstance, GetInstanceOutput, GetInstanceOutputArg, KillInstance, KillInstanceArg,
	SaveInstanceConfigArg, SendInstanceInput, SendInstanceInputArg,
};
use nitro_shared::{
	Side,
	id::{InstanceID, TemplateID},
	io::dir_size,
	output::NitroOutput,
	util::DeserListOrSingle,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::{
	config::{
		Config,
		modifications::{ConfigModification, apply_modifications_and_write},
	},
	instance::{Instance, tracking::RunningInstanceRegistry},
	io::paths::Paths,
	plugin::PluginManager,
};

/// A suggested rate at which to update output from a local instance
pub const LOCAL_INSTANCE_OUTPUT_UPDATE_RATE: Duration = Duration::from_millis(100);
/// A suggested rate at which to update output from a remote instance
pub const REMOTE_INSTANCE_OUTPUT_UPDATE_RATE: Duration = Duration::from_millis(350);

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

		let modifications = vec![ConfigModification::UpdateInstance(self.id.clone(), config)];
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

	/// Extracts a template from this instance
	pub async fn extract(
		&self,
		new_id: &TemplateID,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let mut config = self.original_config.clone();
		config.side = None;
		let config = TemplateConfig {
			instance: config,
			..Default::default()
		};

		let inst_config = InstanceConfig {
			side: Some(self.side()),
			..Default::default()
		};

		let modifications = vec![
			ConfigModification::AddTemplate(new_id.clone(), config),
			ConfigModification::UpdateInstance(self.id.clone(), inst_config),
		];
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

			if path.exists() {
				tokio::fs::remove_dir_all(path).await?;
			}
		}

		Ok(())
	}

	/// Kills all processes for this instance, or a specific account if specified.
	pub async fn kill(
		&self,
		account: Option<&str>,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let mut registry = RunningInstanceRegistry::open(paths)?;
		registry.kill_instance(&self.id, account);
		let _ = registry.write();
		if let Some(source_plugin) = &self.original_config.source_plugin {
			let arg = KillInstanceArg {
				id: self.id.to_string(),
				account: account.map(|x| x.to_string()),
				config: self.config.clone(),
			};
			if let Some(result) = plugins
				.call_hook_on_plugin(KillInstance, source_plugin, &arg, paths, o)
				.await?
			{
				result.result(o).await?;
			}
		}

		Ok(())
	}

	/// Gets the live standard output of this instance, starting from the specified UTF-8 character index
	pub async fn get_output(
		&self,
		after: u64,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<String> {
		if let Some(source_plugin) = &self.original_config.source_plugin
			&& self.config.launch_mode != LaunchMode::Normal
		{
			let arg = GetInstanceOutputArg {
				id: self.id.to_string(),
				config: self.config.clone(),
				after,
			};
			if let Some(result) = plugins
				.call_hook_on_plugin(GetInstanceOutput, source_plugin, &arg, paths, o)
				.await?
			{
				return result.result(o).await;
			}
		} else {
			let registry = RunningInstanceRegistry::open(paths)?;
			if let Some(instance) = registry.get_instance(&self.id, None) {
				if let Some(stdout_file) = &instance.stdout_file {
					let path = paths.internal.join("stdio").join(stdout_file);
					if path.exists() {
						let mut file = tokio::fs::File::open(&path).await?;
						file.seek(SeekFrom::Start(after)).await?;
						let mut output = String::new();
						file.read_to_string(&mut output).await?;
						return Ok(output);
					}
				}
			}
		}

		Ok(String::new())
	}

	/// Sends data to this instance's standard input
	pub async fn send_input(
		&self,
		input: &str,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		if let Some(source_plugin) = &self.original_config.source_plugin
			&& self.config.launch_mode != LaunchMode::Normal
		{
			let arg = SendInstanceInputArg {
				id: self.id.to_string(),
				config: self.config.clone(),
				input: input.to_string(),
			};
			if let Some(result) = plugins
				.call_hook_on_plugin(SendInstanceInput, source_plugin, &arg, paths, o)
				.await?
			{
				result.result(o).await?;
			}
		} else {
			let registry = RunningInstanceRegistry::open(paths)?;
			if let Some(instance) = registry.get_instance(&self.id, None) {
				if let Some(stdin_file) = &instance.stdin_file {
					let path = paths.internal.join("stdio").join(stdin_file);
					if path.exists() {
						let mut file = open_file_append(&path)?;
						file.write_all(input.as_bytes())?;
					}
				}
			}
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

	/// Gets the saves for this instance
	pub fn get_saves(&self) -> anyhow::Result<Vec<InstanceSave>> {
		let Some(dir) = &self.dir else {
			return Ok(Vec::new());
		};

		get_instance_saves(dir, self.side())
	}

	/// Gets the screenshots for this instance
	pub fn get_screenshots(&self) -> anyhow::Result<Vec<PathBuf>> {
		let Some(dir) = &self.dir else {
			return Ok(Vec::new());
		};

		get_instance_screenshots(dir, self.side())
	}

	/// Gets the servers for this instance
	pub fn get_servers(&self) -> anyhow::Result<Vec<ServerInfo>> {
		let Some(dir) = &self.dir else {
			return Ok(Vec::new());
		};

		get_instance_servers(dir, self.side())
	}
}
