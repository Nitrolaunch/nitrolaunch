use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_core::io::files::{create_leading_dirs, update_link};
use nitro_plugin::hooks::{InstanceLaunchArg, UpdateWorldFiles};
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};

use crate::{io::paths::Paths, plugin::PluginManager};

/// Watcher that periodically updates shared world files
pub struct WorldFilesWatcher {
	plugins: PluginManager,
	saves_dir: PathBuf,
	world_files_dir: PathBuf,
	/// Used for comparing differences in the number of saves
	save_count: Option<usize>,
}

impl WorldFilesWatcher {
	/// Creates a new WorldFilesWatcher
	pub fn new(game_dir: &Path, plugins: PluginManager) -> anyhow::Result<Self> {
		let saves_dir = game_dir.join("saves");
		let _ = std::fs::create_dir_all(&saves_dir);
		let world_files_dir = game_dir.join("world_files");
		let _ = std::fs::create_dir_all(&world_files_dir);

		Ok(Self {
			plugins,
			saves_dir,
			world_files_dir,
			save_count: None,
		})
	}

	/// Watches for updates. Should be called periodically while the instance is running
	pub async fn watch(
		&mut self,
		plugin_arg: &InstanceLaunchArg,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let read = self.saves_dir.read_dir()?;
		let save_count = read
			.filter(|x| {
				let Ok(x) = x else {
					return false;
				};

				let Ok(file_type) = x.file_type() else {
					return false;
				};

				file_type.is_dir()
			})
			.count();
		if self.save_count.is_some_and(|x| x < save_count) || self.save_count.is_none() {
			let is_first_update = self.save_count.is_none();
			self.save_count = Some(save_count);
			self.update(is_first_update, plugin_arg, paths, o)
				.await
				.context("Failed to update shared files")?;
		} else {
			self.save_count = Some(save_count);
		}

		Ok(())
	}

	/// Updates shared world files by linking them all to each world
	pub async fn update(
		&mut self,
		is_first_run: bool,
		plugin_arg: &InstanceLaunchArg,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		o.display(
			MessageContents::Simple("Updating shared world files".into()),
			MessageLevel::Debug,
		);

		// Get a list of all the worlds in the directory
		let read = self.saves_dir.read_dir()?;
		let mut save_paths = Vec::new();

		for entry in read {
			let Ok(entry) = entry else {
				continue;
			};

			let Ok(file_type) = entry.file_type() else {
				continue;
			};

			if file_type.is_dir() {
				save_paths.push(entry.path());
			}
		}

		// Collect all of the shared files we need to link
		let mut all_paths = Vec::new();

		fn walk_dir(path: &Path, all_paths: &mut Vec<PathBuf>) -> anyhow::Result<()> {
			let read = path.read_dir()?;

			for entry in read {
				let Ok(entry) = entry else {
					continue;
				};

				let Ok(file_type) = entry.file_type() else {
					continue;
				};

				if file_type.is_file() {
					all_paths.push(entry.path());
				} else {
					walk_dir(&entry.path(), all_paths)?;
				}
			}

			Ok(())
		}

		walk_dir(&self.world_files_dir, &mut all_paths)?;

		for path in all_paths {
			let Ok(relative) = path.strip_prefix(&self.world_files_dir) else {
				continue;
			};

			for save in &save_paths {
				let target_path = save.join(relative);
				let _ = create_leading_dirs(&target_path);
				if is_first_run && target_path.exists() {
					let _ = std::fs::remove_file(&target_path);
				}
				let _ = update_link(&path, &target_path);
			}
		}

		let result = self
			.plugins
			.call_hook(UpdateWorldFiles, plugin_arg, paths, o)
			.await;
		match result {
			Ok(result) => {
				for result in result {
					let result = result.result(o).await;
					if let Err(e) = result {
						o.display(
							MessageContents::Error(format!("{e:?}")),
							MessageLevel::Important,
						);
					}
				}
			}
			Err(_) => {}
		}

		Ok(())
	}
}
