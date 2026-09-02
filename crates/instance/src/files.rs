use std::{
	fs::File,
	io::BufReader,
	path::{Path, PathBuf},
};

use anyhow::Context;
use base64::Engine;
use nbt::decode::read_compound_tag;
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

/// Gets the servers for an instance
pub fn get_instance_servers(instance_dir: &Path, side: Side) -> anyhow::Result<Vec<ServerInfo>> {
	if let Side::Server = side {
		return Ok(Vec::new());
	}

	let servers_file = instance_dir.join("servers.dat");
	if !servers_file.exists() {
		return Ok(Vec::new());
	}

	let mut servers =
		BufReader::new(File::open(servers_file).context("Failed to open servers.dat")?);
	let root = read_compound_tag(&mut servers).context("Failed to parse servers.dat")?;
	let servers = root
		.get_compound_tag_vec("servers")
		.map_err(|e| anyhow::anyhow!("Failed to read servers list: {e}"))?;

	let out = servers
		.iter()
		.filter_map(|s| {
			Some(ServerInfo {
				name: s.get_str("name").ok().map(|x| x.to_string()),
				address: s.get_str("ip").ok()?.to_string(),
				icon_png: s.get_str("icon").ok().and_then(|x| {
					base64::engine::GeneralPurpose::new(
						&base64::alphabet::STANDARD,
						base64::engine::GeneralPurposeConfig::new(),
					)
					.decode(x)
					.ok()
				}),
			})
		})
		.collect();

	Ok(out)
}

/// Basic information about a server in an instance
#[derive(Clone)]
pub struct ServerInfo {
	/// The name of the server, if it exists
	pub name: Option<String>,
	/// The address of the server
	pub address: String,
	/// The icon of the server, if it exists
	pub icon_png: Option<Vec<u8>>,
}
