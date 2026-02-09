use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{bail, Context};
use nitro_core::io::{files::create_leading_dirs, json_from_file, json_to_file};
use nitro_pkg::{PkgRequest, PkgRequestSource};
use nitro_shared::{
	loaders::Loader,
	output::{MessageContents, NitroOutput},
	pkg::ArcPkgReq,
	translate,
};
use serde::{Deserialize, Serialize};

use crate::{
	instance::Instance,
	io::{
		lock::{Lockfile, LockfileAddon, LockfilePackage},
		paths::Paths,
	},
};

impl Instance {
	/// Opens the lockfile for this instance and returns it
	pub fn get_lockfile(
		&mut self,
		global_lock: &Lockfile,
		paths: &Paths,
	) -> anyhow::Result<InstanceLockfile> {
		let lock_path = InstanceLockfile::get_path(self.dir.as_deref(), &self.id, paths);
		let lock = if lock_path.exists() {
			InstanceLockfile::open(&lock_path)?
		} else {
			InstanceLockfile {
				contents: InstanceLockfileContents::from_global(global_lock, &self.id),
				path: lock_path,
			}
		};

		Ok(lock)
	}
}

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
			json_from_file(path).context("Failed to read instance lockfile")?
		} else {
			InstanceLockfileContents::default()
		};

		Ok(Self {
			contents,
			path: path.to_owned(),
		})
	}

	/// Get the path to the lockfile
	pub fn get_path(inst_dir: Option<&Path>, instance_id: &str, paths: &Paths) -> PathBuf {
		if let Some(inst_dir) = inst_dir {
			inst_dir.join("nitro_lock.json")
		} else {
			paths
				.internal
				.join("lock/instances")
				.join(format!("{instance_id}.json"))
		}
	}

	/// Finish using the lockfile and write to the disk
	pub fn write(&self) -> anyhow::Result<()> {
		create_leading_dirs(&self.path)?;
		json_to_file(&self.path, &self.contents).context("Failed to write to lockfile")?;

		Ok(())
	}

	/// Updates a package with a new version.
	/// Returns a list of addon files to be removed
	pub async fn update_package(
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
	/// Returns any addon files that need to be removed from the instance.
	pub fn remove_unused_packages(
		&mut self,
		used_packages: &[ArcPkgReq],
	) -> anyhow::Result<Vec<PathBuf>> {
		let mut pkgs_to_remove = Vec::new();
		for req in self.contents.packages.keys() {
			let req2 = Arc::new(PkgRequest::parse(req, PkgRequestSource::UserRequire));
			if used_packages.contains(&req2) {
				continue;
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
	pub(crate) minecraft_version: Option<String>,
	pub(crate) loader: Loader,
	pub(crate) loader_version: Option<String>,
	packages: HashMap<String, LockfilePackage>,
}

impl InstanceLockfileContents {
	/// Migrates an instance lockfile from the old shared lockfile format
	pub fn from_global(global_lock: &Lockfile, instance_id: &str) -> Self {
		let (minecraft_version, loader, loader_version) =
			if let Some(lock_instance) = global_lock.get_instance(instance_id) {
				(
					Some(lock_instance.version.clone()),
					lock_instance.loader.clone(),
					lock_instance.loader_version.clone(),
				)
			} else {
				(None, Loader::Vanilla, None)
			};

		let packages = global_lock
			.get_instance_packages(instance_id)
			.cloned()
			.unwrap_or_default();

		Self {
			minecraft_version,
			loader,
			loader_version,
			packages,
		}
	}
}
