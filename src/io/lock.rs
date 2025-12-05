use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context};
use nitro_core::io::{json_from_file, json_to_file_pretty};
use nitro_pkg::{PkgRequest, PkgRequestSource};
use nitro_shared::loaders::Loader;
use nitro_shared::output::{MessageContents, NitroOutput};
use nitro_shared::translate;
use serde::{Deserialize, Serialize};

use nitro_shared::addon::{Addon, AddonKind};
use nitro_shared::pkg::{ArcPkgReq, PackageAddonOptionalHashes, PackageID};

use super::paths::Paths;

/// A file that remembers important info like what files and packages are currently installed
#[derive(Debug)]
pub struct Lockfile {
	contents: LockfileContents,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
struct LockfileContents {
	packages: HashMap<String, HashMap<String, LockfilePackage>>,
	instances: HashMap<String, LockfileInstance>,
	/// Instances that have done their first update
	created_instances: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub(crate) struct LockfileInstance {
	pub(crate) version: String,
	pub(crate) loader: Loader,
	pub(crate) loader_version: Option<String>,
}

/// Package stored in the lockfile
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockfilePackage {
	/// The addons of this package
	pub addons: Vec<LockfileAddon>,
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
				.ok_or(anyhow!("Invalid addon kind '{}'", self.kind))?,
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
				fs::remove_file(path).context("Failed to remove addon")?;
			}
		}

		Ok(())
	}
}

impl LockfileContents {
	/// Fix changes in lockfile format
	pub fn fix(&mut self) {
		for (.., instance) in &mut self.packages {
			for (.., package) in instance {
				for addon in &mut package.addons {
					if addon.file_name.is_none() {
						addon.file_name = Some(addon.id.clone())
					}
				}
			}
		}
	}
}

impl Lockfile {
	/// Open the lockfile
	pub fn open(paths: &Paths) -> anyhow::Result<Self> {
		let path = Self::get_path(paths);
		let mut contents = if path.exists() {
			json_from_file(path).context("Failed to open lockfile")?
		} else {
			LockfileContents::default()
		};
		contents.fix();
		Ok(Self { contents })
	}

	/// Get the path to the lockfile
	pub fn get_path(paths: &Paths) -> PathBuf {
		paths.internal.join("lock.json")
	}

	/// Finish using the lockfile and write to the disk
	pub fn finish(&mut self, paths: &Paths) -> anyhow::Result<()> {
		json_to_file_pretty(Self::get_path(paths), &self.contents)
			.context("Failed to write to lockfile")?;

		Ok(())
	}

	/// Get a specific instance from the lockfile
	pub(crate) fn get_instance(&self, instance: &str) -> Option<&LockfileInstance> {
		self.contents.instances.get(instance)
	}

	/// Updates a package with a new version.
	/// Returns a list of addon files to be removed
	pub fn update_package(
		&mut self,
		id: &str,
		instance: &str,
		addons: &[LockfileAddon],
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Vec<PathBuf>> {
		let mut files_to_remove = Vec::new();
		let mut new_files = Vec::new();
		if let Some(instance) = self.contents.packages.get_mut(instance) {
			if let Some(pkg) = instance.get_mut(id) {
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
			} else {
				instance.insert(
					id.to_owned(),
					LockfilePackage {
						addons: addons.to_vec(),
					},
				);
				new_files.extend(addons.iter().flat_map(|x| x.files.clone()));
			}
		} else {
			self.contents
				.packages
				.insert(instance.to_owned(), HashMap::new());
			self.update_package(id, instance, addons, o)?;
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
		instance: &str,
		used_packages: &[ArcPkgReq],
	) -> anyhow::Result<Vec<PathBuf>> {
		if let Some(inst) = self.contents.packages.get_mut(instance) {
			let mut pkgs_to_remove = Vec::new();
			for (req, pkg) in inst.iter() {
				if used_packages.contains(&Arc::new(PkgRequest::parse(
					req,
					PkgRequestSource::UserRequire,
				))) {
					continue;
				}

				// Backwards compatability fix to prevent removing packages that add a repository
				if inst.values().any(|x| x.addons == pkg.addons) {
					continue;
				}

				pkgs_to_remove.push(req.clone());
			}

			let mut files_to_remove = Vec::new();
			for pkg_id in pkgs_to_remove {
				if let Some(pkg) = inst.remove(&pkg_id) {
					for addon in pkg.addons {
						files_to_remove.extend(addon.files.iter().map(PathBuf::from));
					}
				}
			}

			Ok(files_to_remove)
		} else {
			Ok(vec![])
		}
	}

	/// Ensures that an instance is created
	pub fn ensure_instance_created(&mut self, instance: &str, version: &str) {
		if !self.contents.instances.contains_key(instance) {
			self.contents.instances.insert(
				instance.to_string(),
				LockfileInstance {
					version: version.to_string(),
					loader_version: None,
					loader: Loader::Vanilla,
				},
			);
		}
	}

	/// Updates the version of an instance
	pub fn update_instance_version(&mut self, instance: &str, version: &str) -> anyhow::Result<()> {
		if let Some(instance) = self.contents.instances.get_mut(instance) {
			instance.version = version.to_string();
			Ok(())
		} else {
			bail!("Instance {instance} does not exist")
		}
	}

	/// Updates the loader of an instance
	pub fn update_instance_loader(&mut self, instance: &str, loader: Loader) -> anyhow::Result<()> {
		if let Some(instance) = self.contents.instances.get_mut(instance) {
			instance.loader = loader;
			Ok(())
		} else {
			bail!("Instance {instance} does not exist")
		}
	}

	/// Updates the loader version of an instance
	pub fn update_instance_loader_version(
		&mut self,
		instance: &str,
		version: Option<String>,
	) -> anyhow::Result<()> {
		if let Some(instance) = self.contents.instances.get_mut(instance) {
			instance.loader_version = version;
			Ok(())
		} else {
			bail!("Instance {instance} does not exist")
		}
	}

	/// Check whether an instance has done its first update successfully
	pub fn has_instance_done_first_update(&self, instance: &str) -> bool {
		self.contents.created_instances.contains(instance)
	}

	/// Update whether an instance has done its first update
	pub fn update_instance_has_done_first_update(&mut self, instance: &str) {
		self.contents.created_instances.insert(instance.to_string());
	}

	/// Get the locked packages of an instance. Returns None if the instance does not exist.
	pub fn get_instance_packages(
		&self,
		instance: &str,
	) -> Option<&HashMap<String, LockfilePackage>> {
		self.contents.packages.get(instance)
	}
}
