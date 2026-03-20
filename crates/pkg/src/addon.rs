use std::path::PathBuf;

use nitro_instance::addon::Addon;
use nitro_shared::{minecraft::AddonKind, pkg::PackageAddonOptionalHashes};

use crate::PackageID;

/// Some content that is installed on Minecraft
#[derive(Debug, Clone)]
pub struct PackageAddon {
	/// What type of addon this is
	pub kind: AddonKind,
	/// The ID of this addon, unique among a package
	pub id: String,
	/// The addon's file name
	pub file_name: String,
	/// The ID of the package that installed this addon
	pub pkg_id: PackageID,
	/// Version of the addon, used for caching
	pub version: Option<String>,
	/// Hashes of the addon
	pub hashes: PackageAddonOptionalHashes,
}

impl PackageAddon {
	/// Gets the target addon represented by this package addon
	pub fn addon(&self, storage_path: PathBuf) -> Addon {
		Addon {
			kind: self.kind,
			file_name: self.file_name.clone(),
			path: None,
			source: Some(storage_path),
		}
	}
}

/// Checks for a valid addon version identifier that is compatible with all systems
pub fn is_addon_version_valid(version: &str) -> bool {
	if !version.is_ascii() {
		return false;
	}

	// Can be exploited to create escapes in paths
	if version.contains("..") {
		return false;
	}

	for c in version.chars() {
		if !c.is_ascii_alphanumeric() && c != '-' && c != '+' && c != '.' {
			return false;
		}
	}

	true
}
