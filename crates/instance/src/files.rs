use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_shared::Side;

/// Basic information about a save in an instance
#[derive(Clone)]
pub struct InstanceSave {
	/// The name of the save
	pub name: String,
	/// The path to the save's icon, if it exists
	pub icon_path: Option<PathBuf>,
}

/// Gets the saves for an instance
pub fn get_instance_saves(instance_dir: &Path, side: Side) -> anyhow::Result<Vec<InstanceSave>> {
	if let Side::Server = side {
		let path = instance_dir.join("world");
		if path.exists() && path.is_dir() {
			return Ok(vec![process_save(&path)?]);
		} else {
			return Ok(Vec::new());
		}
	}

	let saves_dir = instance_dir.join("saves");

	if !saves_dir.exists() {
		return Ok(Vec::new());
	}

	let mut saves = Vec::new();
	for entry in std::fs::read_dir(saves_dir)? {
		let Ok(entry) = entry else {
			continue;
		};
		if entry.file_type().is_ok_and(|x| x.is_dir()) {
			let Ok(save) = process_save(&entry.path()) else {
				continue;
			};
			saves.push(save);
		}
	}

	Ok(saves)
}

fn process_save(dir: &Path) -> anyhow::Result<InstanceSave> {
	let name = dir
		.file_name()
		.context("Failed to get save directory name")?
		.to_string_lossy()
		.to_string();
	let icon_path = dir.join("icon.png");
	let icon_path = Some(icon_path).filter(|p| p.exists());
	Ok(InstanceSave { name, icon_path })
}

/// Gets the screenshots for an instance
pub fn get_instance_screenshots(instance_dir: &Path, side: Side) -> anyhow::Result<Vec<PathBuf>> {
	if let Side::Server = side {
		return Ok(Vec::new());
	}

	let screenshots_dir = instance_dir.join("screenshots");

	if !screenshots_dir.exists() {
		return Ok(Vec::new());
	}

	let mut screenshots = Vec::new();
	for entry in std::fs::read_dir(screenshots_dir)? {
		let Ok(entry) = entry else {
			continue;
		};
		if entry.file_type().is_ok_and(|x| x.is_file()) {
			screenshots.push(entry.path());
		}
	}

	Ok(screenshots)
}
