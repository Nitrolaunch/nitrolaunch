use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
use nitro_core::io::{json_from_file, json_to_file_pretty};
use serde::{Deserialize, Serialize};

use super::paths::Paths;

/// A file that remembers important info like what files and packages are currently installed
#[derive(Debug)]
pub struct Lockfile {
	contents: LockfileContents,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
struct LockfileContents {
	/// Instances that have done their first update
	created_instances: HashSet<String>,
}

impl Lockfile {
	/// Open the lockfile
	pub fn open(paths: &Paths) -> anyhow::Result<Self> {
		let path = Self::get_path(paths);
		let contents = if path.exists() {
			json_from_file(path).context("Failed to open lockfile")?
		} else {
			LockfileContents::default()
		};
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

	/// Check whether an instance has done its first update successfully
	pub fn has_instance_done_first_update(&self, instance: &str) -> bool {
		self.contents.created_instances.contains(instance)
	}

	/// Update whether an instance has done its first update
	pub fn update_instance_has_done_first_update(&mut self, instance: &str) {
		self.contents.created_instances.insert(instance.to_string());
	}
}
