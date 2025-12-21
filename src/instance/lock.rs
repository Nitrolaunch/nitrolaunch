use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{bail, Context};
use nitro_core::io::{json_from_file, json_to_file_pretty};
use nitro_pkg::{PkgRequest, PkgRequestSource};
use nitro_shared::{
	addon::{Addon, AddonKind},
	loaders::Loader,
	output::{MessageContents, NitroOutput},
	pkg::{ArcPkgReq, PackageAddonOptionalHashes, PackageID},
	translate,
};
use serde::{Deserialize, Serialize};

/// Stored install info about an instance
pub struct InstanceLockfile {
	contents: InstanceLockfileContents,
	path: PathBuf,
}

impl InstanceLockfile {
	/// Open the lockfile
	pub fn open(inst_dir: &Path) -> anyhow::Result<Self> {
		let path = Self::get_path(inst_dir);
		let contents: InstanceLockfileContents = if path.exists() {
			json_from_file(&path).context("Failed to read instance lockfile")?
		} else {
			InstanceLockfileContents::default()
		};

		Ok(Self { contents, path })
	}

	/// Get the path to the lockfile
	pub fn get_path(inst_dir: &Path) -> PathBuf {
		inst_dir.join("nitro_lock.json")
	}

	/// Finish using the lockfile and write to the disk
	pub fn write(&self) -> anyhow::Result<()> {
		json_to_file_pretty(&self.path, &self.contents).context("Failed to write to lockfile")?;

		Ok(())
	}

	/// Updates a package with a new version.
	/// Returns a list of addon files to be removed
	pub fn update_package(
		&mut self,
		req: &PkgRequest,
		addons: &[LockfileAddon],
		content_version: Option<String>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Vec<PathBuf>> {
		let mut files_to_remove = Vec::new();
		let mut new_files = Vec::new();
		let req = req.to_string();
		if let Some(pkg) = self.contents.packages.get_mut(&req) {
			let mut indices = Vec::new();
			// Check for addons that need to be removed
			for (i, current) in pkg.addons.iter().enumerate() {
				if !addons.iter().any(|x| x.id == current.id) {
					indices.push(i);
					files_to_remove.extend(current.files.iter().map(PathBuf::from));
				}
			}
			for i in indices {
				pkg.addons.remove(i);
			}
			// Check for addons that need to be updated
			for requested in addons {
				if let Some(current) = pkg.addons.iter().find(|x| x.id == requested.id) {
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

			pkg.addons = addons.to_vec();
			pkg.content_version = content_version;
		} else {
			self.contents.packages.insert(
				req.clone(),
				LockfilePackage {
					addons: addons.to_vec(),
					content_version,
				},
			);
			new_files.extend(addons.iter().flat_map(|x| x.files.clone()));
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
					.context("Prompt failed")?;

				if !allow {
					bail!("File '{file}' would be overwritten by an addon");
				}
			}
		}

		Ok(files_to_remove)
	}

	/// Remove any unused packages for an instance.
	/// Returns any addon files that need to be removed from the instance.
	pub fn remove_unused_packages(
		&mut self,
		used_packages: &[ArcPkgReq],
	) -> anyhow::Result<Vec<PathBuf>> {
		let mut pkgs_to_remove = Vec::new();
		for (req, pkg) in &self.contents.packages {
			let req2 = Arc::new(PkgRequest::parse(req, PkgRequestSource::UserRequire));
			if used_packages.contains(&req2) {
				continue;
			}

			// Backwards compatability fix to prevent removing packages that add a repository
			if req2.repository.is_some() {
				if self
					.contents
					.packages
					.values()
					.any(|x| x.addons == pkg.addons)
				{
					continue;
				}
			}

			pkgs_to_remove.push(req.clone());
		}

		let mut files_to_remove = Vec::new();
		for pkg_id in pkgs_to_remove {
			if let Some(pkg) = self.contents.packages.remove(&pkg_id) {
				for addon in pkg.addons {
					files_to_remove.extend(addon.files.iter().map(PathBuf::from));
				}
			}
		}

		Ok(files_to_remove)
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

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct InstanceLockfileContents {
	pub(crate) minecraft_version: Option<String>,
	pub(crate) loader: Loader,
	pub(crate) loader_version: Option<String>,
	packages: HashMap<String, LockfilePackage>,
}

/// Package stored in the lockfile
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockfilePackage {
	/// The addons of this package
	pub addons: Vec<LockfileAddon>,
	/// The selected content version of this package
	pub content_version: Option<String>,
}

/// Format for an addon in the lockfile
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct LockfileAddon {
	#[serde(alias = "name")]
	id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	file_name: Option<String>,
	files: Vec<String>,
	kind: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	version: Option<String>,
	#[serde(default)]
	#[serde(skip_serializing_if = "PackageAddonOptionalHashes::is_empty")]
	hashes: PackageAddonOptionalHashes,
}

impl LockfileAddon {
	/// Converts an addon to the format used by the lockfile.
	/// Paths is the list of paths for the addon in the instance
	pub fn from_addon(addon: &Addon, paths: Vec<PathBuf>) -> Self {
		Self {
			id: addon.id.clone(),
			file_name: Some(addon.file_name.clone()),
			files: paths
				.iter()
				.map(|x| {
					x.to_str()
						.expect("Failed to convert addon path to a string")
						.to_owned()
				})
				.collect(),
			kind: addon.kind.to_string(),
			version: addon.version.clone(),
			hashes: addon.hashes.clone(),
		}
	}

	/// Converts this LockfileAddon to an Addon
	pub fn to_addon(&self, pkg_id: PackageID) -> anyhow::Result<Addon> {
		Ok(Addon {
			kind: AddonKind::parse_from_str(&self.kind)
				.with_context(|| format!("Invalid addon kind '{}'", self.kind))?,
			id: self.id.clone(),
			file_name: self
				.file_name
				.clone()
				.expect("Filename should have been filled in or fixed"),
			pkg_id,
			version: self.version.clone(),
			hashes: self.hashes.clone(),
		})
	}

	/// Remove this addon
	pub fn remove(&self) -> anyhow::Result<()> {
		for file in self.files.iter() {
			let path = PathBuf::from(file);
			if path.exists() {
				std::fs::remove_file(path).context("Failed to remove addon")?;
			}
		}

		Ok(())
	}
}
