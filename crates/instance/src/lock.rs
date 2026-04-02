use std::{
	collections::HashMap,
	fs::File,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{bail, Context};
use nitro_shared::{
	loaders::Loader,
	minecraft::AddonKind,
	output::{MessageContents, NitroOutput},
	pkg::{AddonOptionalHashes, ArcPkgReq, PkgRequest, PkgRequestSource},
	translate,
};
use serde::{Deserialize, Serialize};

use crate::addon::Addon;

/// Stored install info about an instance
#[derive(Debug)]
pub struct InstanceLockfile {
	contents: InstanceLockfileContents,
	path: PathBuf,
}

impl InstanceLockfile {
	/// Open the lockfile at the specified path
	pub fn open(path: &Path) -> anyhow::Result<Self> {
		let contents: InstanceLockfileContents = if path.exists() {
			serde_json::from_reader(File::open(path)?)
				.context("Failed to read instance lockfile")?
		} else {
			InstanceLockfileContents::default()
		};

		Ok(Self {
			contents,
			path: path.to_owned(),
		})
	}

	/// Get the path to the lockfile
	pub fn get_path(inst_dir: Option<&Path>, instance_id: &str, internal_dir: &Path) -> PathBuf {
		if let Some(inst_dir) = inst_dir {
			inst_dir.join("nitro_lock.json")
		} else {
			internal_dir
				.join("lock/instances")
				.join(format!("{instance_id}.json"))
		}
	}

	/// Finish using the lockfile and write to the disk
	pub fn write(&self) -> anyhow::Result<()> {
		if let Some(parent) = self.path.parent() {
			let _ = std::fs::create_dir_all(parent);
		}
		serde_json::to_writer(File::create(&self.path)?, &self.contents)
			.context("Failed to write to lockfile")?;

		Ok(())
	}

	/// Updates a package with a new version.
	/// Returns a list of files to be removed
	pub async fn update_package(
		&mut self,
		req: &PkgRequest,
		addons: &[LockfileAddon],
		content_version: Option<String>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Vec<PathBuf>> {
		let mut files_to_remove = Vec::new();
		let mut new_files = Vec::new();
		let req = req.to_string_no_version();

		// Update the package
		if let Some(pkg) = self.contents.packages.get_mut(&req) {
			pkg.content_version = content_version;
		} else {
			self.contents
				.packages
				.insert(req.clone(), LockfilePackage { content_version });
		}

		// Remove all addons for the package currently in the list, and remove files that aren't in the package anymore
		self.contents.addons.retain(|addon| {
			if !addon.is_from_package(&req) {
				return true;
			}

			if !addons.iter().any(|x| x.id == addon.id) {
				files_to_remove.extend(addon.to_addon().target_paths.clone());
			}

			false
		});

		// Update addon files
		for requested in addons {
			let Some(addon_id) = &requested.id else {
				continue;
			};

			if let Some(current) = self
				.contents
				.addons
				.iter()
				.find(|x| x.is_package_addon(&req, addon_id))
			{
				files_to_remove.extend(
					current
						.files
						.iter()
						.filter(|x| !requested.files.contains(x))
						.map(PathBuf::from),
				);
				new_files.extend(
					requested
						.files
						.iter()
						.filter(|x| !current.files.contains(x))
						.cloned(),
				);
			} else {
				new_files.extend(requested.files.clone());
			};
		}

		// Add new addons
		for requested in addons {
			self.contents.addons.push(requested.clone());
		}

		for file in &new_files {
			if PathBuf::from(file).exists() && !file.contains("nitro_") {
				let allow = o
					.prompt_yes_no(
						false,
						MessageContents::Warning(translate!(
							o,
							OverwriteAddonFilePrompt,
							"file" = file
						)),
					)
					.await
					.context("Prompt failed")?;

				if !allow {
					bail!("File '{file}' would be overwritten by an addon");
				}
			}
		}

		Ok(files_to_remove)
	}

	/// Remove any unused packages for an instance.
	/// Returns any addons that need to be removed from the instance.
	pub fn remove_unused_packages(
		&mut self,
		used_packages: &[ArcPkgReq],
	) -> anyhow::Result<Vec<Addon>> {
		let mut pkgs_to_remove = Vec::new();
		for req in self.contents.packages.keys() {
			let req2 = Arc::new(PkgRequest::parse(req, PkgRequestSource::UserRequire));
			if used_packages.contains(&req2) {
				continue;
			}

			pkgs_to_remove.push(req.clone());
		}

		let mut addons_to_remove = Vec::new();
		for pkg_id in pkgs_to_remove {
			self.contents.packages.remove(&pkg_id);
			for addon in &self.contents.addons {
				if addon.is_from_package(&pkg_id) {
					addons_to_remove.push(addon.to_addon());
				}
			}
		}

		Ok(addons_to_remove)
	}

	/// Gets the current Minecraft version
	pub fn get_minecraft_version(&self) -> Option<&String> {
		self.contents.minecraft_version.as_ref()
	}

	/// Gets the current loader
	pub fn get_loader(&self) -> &Loader {
		&self.contents.loader
	}

	/// Gets the current loader version
	pub fn get_loader_version(&self) -> Option<&String> {
		self.contents.loader_version.as_ref()
	}

	/// Updates the Minecraft version
	pub fn update_minecraft_version(&mut self, version: &str) {
		self.contents.minecraft_version = Some(version.to_string());
	}

	/// Updates the loader
	pub fn update_loader(&mut self, loader: Loader) {
		self.contents.loader = loader;
	}

	/// Updates the loader version
	pub fn update_loader_version(&mut self, version: Option<String>) {
		self.contents.loader_version = version;
	}

	/// Get the locked packages
	pub fn get_packages(&self) -> &HashMap<String, LockfilePackage> {
		&self.contents.packages
	}
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct InstanceLockfileContents {
	/// The currently installed Minecraft version of the instance
	pub minecraft_version: Option<String>,
	/// The currently installed loader of the instance
	pub loader: Loader,
	/// The currently installed loader version of the instance
	pub loader_version: Option<String>,
	/// Currently installed packages for the instance
	pub packages: HashMap<String, LockfilePackage>,
	/// Currently installed addons on the instance
	#[serde(default)]
	pub addons: Vec<LockfileAddon>,
}

/// Package stored in the instance lockfile
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockfilePackage {
	/// The selected content version of this package
	pub content_version: Option<String>,
}

/// Addon stored in the instance lockfile
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct LockfileAddon {
	/// ID of the addon
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// Source package for this addon
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub package: Option<String>,
	/// Whether this addon was from a modpack
	#[serde(default)]
	pub from_modpack: bool,
	/// Filename of the addon
	pub file_name: String,
	/// Files for the addon
	pub files: Vec<String>,
	/// The kind of the addon
	pub kind: AddonKind,
	/// Hashes for the addon
	#[serde(default)]
	#[serde(skip_serializing_if = "AddonOptionalHashes::is_empty")]
	pub hashes: AddonOptionalHashes,
}

impl LockfileAddon {
	/// Checks if this is addon is from a specific package
	pub fn is_from_package(&self, req: &str) -> bool {
		self.package.as_ref().is_some_and(|x| x == req)
	}

	/// Checks if this is a specific addon from a specific package
	pub fn is_package_addon(&self, req: &str, addon_id: &str) -> bool {
		self.is_from_package(req) && self.id.as_ref().is_some_and(|x| x == addon_id)
	}

	/// Converts this lockfile addon to an addon
	pub fn to_addon(&self) -> Addon {
		Addon {
			kind: self.kind,
			file_name: self.file_name.clone(),
			original_path: None,
			target_paths: self.files.iter().map(PathBuf::from).collect(),
			source: None,
			hashes: self.hashes.clone(),
		}
	}
}
