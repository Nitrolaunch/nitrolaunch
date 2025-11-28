use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::files::paths::Paths;
use super::{json_from_file, json_to_file_pretty};

/// A file that remembers important info like what versions and files are currently installed
#[derive(Debug)]
pub struct PersistentData {
	contents: PersistentDataContents,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
struct PersistentDataContents {
	/// Maps of Java types to maps between major version and installation info
	java: HashMap<String, HashMap<String, PersistentDataJavaVersion>>,
}

/// Info about an installed major version for a Java type
#[derive(Serialize, Deserialize, Debug)]
struct PersistentDataJavaVersion {
	version: String,
	path: String,
}

impl PersistentDataContents {
	/// Fix changes in persistent data format
	pub fn fix(&mut self) {}
}

impl PersistentData {
	/// Open the persistent data file
	pub fn open(paths: &Paths) -> anyhow::Result<Self> {
		let path = Self::get_path(paths);
		let mut contents = if path.exists() {
			json_from_file(&path).context("Failed to get JSON contents")?
		} else {
			PersistentDataContents::default()
		};
		contents.fix();
		Ok(Self { contents })
	}

	/// Get the path to the persistent data file
	pub fn get_path(paths: &Paths) -> PathBuf {
		paths.internal.join("core_persistent.json")
	}

	/// Finish using the persistent data file and write to the disk
	pub async fn dump(&mut self, paths: &Paths) -> anyhow::Result<()> {
		let path = Self::get_path(paths);
		json_to_file_pretty(path, &self.contents)
			.context("Failed to write persistent data contents")?;

		Ok(())
	}

	/// Updates a Java installation with a new version. Returns true if the version has changed.
	pub(crate) fn update_java_installation(
		&mut self,
		java: &str,
		major_version: &str,
		version: &str,
		path: &Path,
	) -> anyhow::Result<bool> {
		let installation = self.contents.java.entry(java.to_string()).or_default();
		let path_str = path.to_string_lossy().to_string();
		if let Some(current_version) = installation.get_mut(major_version) {
			if current_version.version == version {
				// Even if the version is the same we want to update the path to prevent infinite installations (since the dir might not exist)
				current_version.path = path_str;
				Ok(false)
			} else {
				// Remove the old installation, if it exists
				let current_version_path = PathBuf::from(&current_version.path);
				if current_version_path.exists() {
					fs::remove_dir_all(current_version_path)
						.context("Failed to remove old Java installation")?;
				}
				current_version.version = version.to_string();
				current_version.path = path_str;
				Ok(true)
			}
		} else {
			installation.insert(
				major_version.to_string(),
				PersistentDataJavaVersion {
					version: version.to_string(),
					path: path_str,
				},
			);
			Ok(true)
		}
	}

	/// Gets the path to a Java installation
	pub(crate) fn get_java_path(&self, installation: &str, version: &str) -> Option<PathBuf> {
		let installation = self.contents.java.get(installation)?;
		let version = installation.get(version)?;
		Some(PathBuf::from(version.path.clone()))
	}
}
