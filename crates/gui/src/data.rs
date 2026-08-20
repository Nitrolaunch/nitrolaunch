use std::collections::HashSet;
use std::{collections::HashMap, path::PathBuf};

use nitrolaunch::config_crate::ConfigKind;
use nitrolaunch::core::io::{json_from_file, json_to_file};
use nitrolaunch::io::paths::Paths;
use serde::{Deserialize, Serialize};

use crate::output::SerializableResolutionError;

/// Stored launcher data
#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct LauncherData {
	/// Whether the launcher has been opened before
	pub launcher_opened_before: bool,
	/// Saved icons for instances
	pub saved_instance_icons: Vec<InstanceIcon>,
	/// Set of pinned instances
	#[serde(alias = "pinned")]
	pub pinned_instances: HashSet<String>,
	/// The currently selected account
	#[serde(alias = "current_user")]
	pub current_account: Option<String>,
	/// The last selected package repository
	pub last_repository: Option<String>,
	/// The last package resolution error associated with instances
	pub last_resolution_errors: HashMap<String, SerializableResolutionError>,
	/// The instance or template where a package was last added to
	pub last_added_package: Option<(String, ConfigKind)>,
	/// The instance or template that was last opened
	pub last_opened_instance: Option<(String, ConfigKind)>,
	/// The currently selected base theme
	#[serde(alias = "theme")]
	pub base_theme: Option<String>,
	/// The currently selected overlay themes
	pub overlay_themes: Vec<String>,
	/// The current zoom level of the app
	#[serde(default = "default_zoom")]
	pub zoom: f64,
}

fn default_zoom() -> f64 {
	1.0
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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum InstanceIcon {
	/// A custom user icon at a path
	File(PathBuf),
}
