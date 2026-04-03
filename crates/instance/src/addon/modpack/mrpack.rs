use std::{
	fs::File,
	io::{Read, Seek},
	path::{Path, PathBuf},
};

use anyhow::Context;
use nitro_shared::{minecraft::AddonKind, pkg::AddonOptionalHashes, Side};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::addon::{
	modpack::{DefaultLinkMethod, LinkMethod, Modpack},
	storage, Addon,
};

/// Modrinth modpack
pub struct ModrinthPack<R> {
	index: ModrinthIndex,
	zip: ZipArchive<R>,
	link_method: Box<dyn LinkMethod + Send + 'static>,
}

#[async_trait::async_trait]
impl<R: Read + Seek + Send + 'static> Modpack<R> for ModrinthPack<R> {
	type Index = ModrinthIndex;

	fn from_stream(r: R) -> anyhow::Result<Self> {
		let mut zip = ZipArchive::new(r).context("Failed to open pack zip file")?;
		let index = zip
			.by_name("modrinth.index.json")
			.context("Failed to open Modrinth index")?;
		let index: ModrinthIndex =
			serde_json::from_reader(index).context("Failed to deserialize index")?;

		Ok(Self {
			index,
			zip,
			link_method: Box::new(DefaultLinkMethod),
		})
	}

	fn index(&self) -> &Self::Index {
		&self.index
	}

	#[cfg(feature = "net")]
	async fn download(
		&mut self,
		addons_dir: &Path,
		client: &nitro_net::download::Client,
	) -> anyhow::Result<()> {
		let mut tasks = tokio::task::JoinSet::new();
		for file in &self.index.files {
			let path = storage::get_sha256_addon_path(addons_dir, &file.hashes.sha512);
			if path.exists() {
				continue;
			}

			let Some(url) = file.downloads.first() else {
				continue;
			};
			let url = url.clone();
			let client = client.clone();
			tasks.spawn(async move { nitro_net::download::file(url, path, &client).await });
		}

		while let Some(result) = tasks.join_next().await {
			result??;
		}

		Ok(())
	}

	fn apply(
		&mut self,
		target: &Path,
		addons_dir: &Path,
		side: Side,
		mut old_pack: Option<&mut Self>,
	) -> anyhow::Result<()> {
		// Link mods and other addons
		for file in &self.index.files {
			let source_path = storage::get_sha256_addon_path(addons_dir, &file.hashes.sha512);
			let target_path = target.join(&file.path);

			let target_path = target.join(target_path);
			if let Some(parent) = target_path.parent() {
				let _ = std::fs::create_dir_all(parent);
			}

			self.link_method
				.link(&source_path, &target_path)
				.context("Failed to link addon")?;
		}

		// Apply overrides
		for i in 0..self.zip.len() {
			let mut file = self.zip.by_index(i)?;
			if file.is_dir() {
				continue;
			}
			let Some(name) = file.enclosed_name() else {
				continue;
			};

			let target_rel_path = if let Ok(path) = name.strip_prefix("overrides/") {
				path
			} else if let Ok(path) = name.strip_prefix("client-overrides/") {
				if side != Side::Client {
					continue;
				}
				path
			} else if let Ok(path) = name.strip_prefix("server-overrides/") {
				if side != Side::Server {
					continue;
				}
				path
			} else {
				continue;
			};

			let target_path = target.join(&target_rel_path);

			if target_path.exists() {
				// If this was an override in the old pack that hasn't changed on the filesystem, we will let it update.
				if let Some(old_pack) = old_pack.as_mut() {
					if old_pack
						.zip
						.file_names()
						.any(|x| x == &name.to_string_lossy())
					{
						let current_data = std::fs::read(&target_path)
							.context("Failed to read existing override file")?;

						let mut old_file = old_pack
							.zip
							.by_name(&name.to_string_lossy())
							.context("Failed to read old override file")?;
						let mut old_data = Vec::with_capacity(old_file.size() as usize);
						old_file
							.read_to_end(&mut old_data)
							.context("Failed to read old override file")?;

						if old_data != current_data {
							continue;
						}
					} else {
						continue;
					}
				} else {
					continue;
				}
			}

			if let Some(parent) = target_path.parent() {
				let _ = std::fs::create_dir_all(parent);
			}
			let mut target_file = File::create(target_path)?;

			std::io::copy(&mut file, &mut target_file).context("Failed to copy file")?;
		}

		Ok(())
	}

	fn get_addons(&mut self, target: &Path, addons_dir: &Path) -> anyhow::Result<Vec<Addon>> {
		let mut out = Vec::new();
		for file in &self.index.files {
			let source_path = storage::get_sha256_addon_path(addons_dir, &file.hashes.sha512);
			let target_path = target.join(&file.path);

			let kind = if file.path.starts_with("mods") {
				AddonKind::Mod
			} else if file.path.starts_with("resourcepacks") {
				AddonKind::ResourcePack
			} else {
				AddonKind::Mod
			};

			let addon = Addon {
				kind,
				file_name: target_path
					.file_name()
					.unwrap()
					.to_string_lossy()
					.to_string(),
				original_path: None,
				target_paths: vec![target_path],
				source: Some(source_path),
				hashes: AddonOptionalHashes {
					sha256: Some(file.hashes.sha512.clone()),
					sha512: None,
				},
			};

			out.push(addon);
		}

		Ok(out)
	}
}

impl<R: Read + Seek> ModrinthPack<R> {
	/// Gets the overrides as relative paths
	pub fn get_overrides(&mut self, side: Side) -> anyhow::Result<Vec<PathBuf>> {
		let mut out = Vec::new();
		for i in 0..self.zip.len() {
			let file = self.zip.by_index(i)?;
			if file.is_dir() {
				continue;
			}
			let Some(name) = file.enclosed_name() else {
				continue;
			};

			if let Ok(path) = name.strip_prefix("overrides/") {
				out.push(path.to_owned());
			} else if let Ok(path) = name.strip_prefix("client-overrides/") {
				if side != Side::Client {
					continue;
				}
				out.push(path.to_owned());
			} else if let Ok(path) = name.strip_prefix("server-overrides/") {
				if side != Side::Server {
					continue;
				}
				out.push(path.to_owned());
			};
		}

		Ok(out)
	}

	/// Replace the link method of this pack
	pub fn set_link_method(&mut self, method: Box<dyn LinkMethod + Send + 'static>) {
		self.link_method = method;
	}
}

/// Information file for a Modrinth pack
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndex {
	/// Name of the modpack
	pub name: String,
	/// Version of the modpack
	pub version_id: String,
	/// Short description of the modpack
	#[serde(default)]
	pub summary: Option<String>,
	/// Files in the pack
	pub files: Vec<ModrinthPackFile>,
	/// Environment dependencies
	pub dependencies: ModrinthPackDependencies,
}

/// File in the Modrinth pack index
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthPackFile {
	/// Path of the file relative to the inst dir
	pub path: String,
	/// Hashes for the file
	pub hashes: ModrinthHashes,
	/// Environment requirements
	#[serde(default)]
	pub env: Option<ModrinthFileEnv>,
	/// URLs for the file
	pub downloads: Vec<String>,
}

impl ModrinthPackFile {
	/// Gets the Modrinth project and version ID of this file, if it is available
	pub fn get_modrinth_info(&self) -> (Option<&str>, Option<&str>) {
		for url in &self.downloads {
			if !url.contains("modrinth") {
				continue;
			}

			// The URL looks like https://cdn.modrinth.com/data/<project>/versions/<version>/<filename>
			let Some(data_pos) = url.find("data/") else {
				continue;
			};
			let slice = &url[data_pos + 5..];
			let mut split = slice.split("/");
			let project = split.next();
			let version = split.nth(1);

			return (project, version);
		}

		(None, None)
	}
}

/// Hashes for a Modrinth pack file
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthHashes {
	/// SHA-512 hash
	pub sha512: String,
}

/// Environment requirements for a Modrinth pack file
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthFileEnv {
	/// Client support
	pub client: SideSupport,
	/// Server support
	pub server: SideSupport,
}

/// Modrinth pack environment dependencies
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModrinthPackDependencies {
	/// Required Minecraft version
	pub minecraft: String,
	/// Required Forge loader version
	#[serde(default)]
	pub forge: Option<String>,
	/// Required NeoForged loader version
	#[serde(default)]
	pub neoforge: Option<String>,
	/// Required Fabric loader version
	#[serde(default)]
	pub fabric_loader: Option<String>,
	/// Required Quilt loader version
	#[serde(default)]
	pub quilt_loader: Option<String>,
}

/// Support status for a file on a specific side
#[derive(Deserialize, Serialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum SideSupport {
	/// Required to be on this side
	Required,
	/// Can optionally be on this side
	Optional,
	/// Unsupported on this side
	Unsupported,
	/// Support unknown
	#[default]
	Unknown,
}
