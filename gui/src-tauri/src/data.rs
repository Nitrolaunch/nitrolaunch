use std::collections::HashSet;
use std::{collections::HashMap, path::PathBuf};

use nitrolaunch::core::io::{json_from_file, json_to_file};
use nitrolaunch::io::paths::Paths;
use serde::{Deserialize, Serialize};

use crate::commands::instance::InstanceOrTemplate;
use crate::output::SerializableResolutionError;

/// Stored launcher data
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LauncherData {
	/// Whether the launcher has been opened before
	pub launcher_opened_before: bool,
	/// Saved icons for instances
	pub saved_instance_icons: Vec<InstanceIcon>,
	/// Set of pinned instances
	pub pinned: HashSet<String>,
	/// The currently selected user
	pub current_user: Option<String>,
	/// The last selected package repository
	pub last_repository: Option<String>,
	/// The last package resolution error associated with instances
	pub last_resolution_errors: HashMap<String, SerializableResolutionError>,
	/// The launch event associated with instances
	pub last_launches: HashMap<String, InstanceLaunch>,
	/// The instance or template where a package was last added to
	pub last_added_package: Option<(String, InstanceOrTemplate)>,
	/// The instance or template that was last opened
	pub last_opened_instance: Option<(String, InstanceOrTemplate)>,
	/// The currently selected theme
	pub theme: Option<String>,
}

impl LauncherData {
	/// Open the launcher data
	pub fn open(paths: &Paths) -> anyhow::Result<Self> {
		let path = Self::path(paths);
		if path.exists() {
			json_from_file(path)
		} else {
			Ok(Self::default())
		}
	}

	/// Write the launcher data
	pub fn write(&self, paths: &Paths) -> anyhow::Result<()> {
		json_to_file(Self::path(paths), &self)
	}

	/// Get the path to the launcher file
	pub fn path(paths: &Paths) -> PathBuf {
		paths.internal.join("launcher_data.json")
	}
}

/// Different icons for instances
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum InstanceIcon {
	/// A custom user icon at a path
	File(PathBuf),
}

/// Data for a single launch of an instance
#[derive(Serialize, Deserialize)]
pub struct InstanceLaunch {
	/// The stdout file of the instance
	pub stdout: String,
	/// The stdin file of the instance
	#[serde(default)]
	pub stdin: Option<String>,
}
