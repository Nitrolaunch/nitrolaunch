use std::fmt::Display;

use nitro_shared::{
	minecraft::VersionManifest,
	versions::{MinecraftLatestVersion, MinecraftVersionDeser, VersionName},
};

/// User-supplied Minecraft version pattern
#[derive(Debug, Clone)]
pub enum MinecraftVersion {
	/// A generic version
	Version(VersionName),
	/// The latest release version available
	Latest,
	/// The latest release or development version available
	LatestSnapshot,
}

impl MinecraftVersion {
	/// Converts a deserialized version to a version
	pub fn from_deser(version: &MinecraftVersionDeser) -> Self {
		match version {
			MinecraftVersionDeser::Version(version) => Self::Version(version.clone()),
			MinecraftVersionDeser::Latest(MinecraftLatestVersion::Release) => Self::Latest,
			MinecraftVersionDeser::Latest(MinecraftLatestVersion::Snapshot) => Self::LatestSnapshot,
		}
	}

	/// Get the correct version from the version manifest
	pub fn get_version(&self, manifest: &VersionManifest) -> Option<VersionName> {
		match self {
			Self::Version(version) => Some(version.clone()),
			Self::Latest => manifest.latest.as_ref().map(|x| x.release.clone()),
			Self::LatestSnapshot => manifest.latest.as_ref().map(|x| x.snapshot.clone()),
		}
	}

	/// Gets the serialized version of this Minecraft version
	pub fn to_serialized(self) -> MinecraftVersionDeser {
		match self {
			Self::Latest => MinecraftVersionDeser::Latest(MinecraftLatestVersion::Release),
			Self::LatestSnapshot => MinecraftVersionDeser::Latest(MinecraftLatestVersion::Snapshot),
			Self::Version(version) => MinecraftVersionDeser::Version(version),
		}
	}
}

impl Display for MinecraftVersion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Version(version) => version,
				Self::Latest => "Latest",
				Self::LatestSnapshot => "Latest Snapshot",
			}
		)
	}
}

#[cfg(test)]
mod tests {
	use serde::Deserialize;

	use super::*;

	#[test]
	fn test_minecraft_version_deserialization() {
		#[derive(Deserialize)]
		struct Test {
			version: MinecraftVersionDeser,
		}

		assert_eq!(
			serde_json::from_str::<Test>(r#"{"version": "1.19"}"#)
				.unwrap()
				.version,
			MinecraftVersionDeser::Version("1.19".into())
		);

		assert_eq!(
			serde_json::from_str::<Test>(r#"{"version": "latest"}"#)
				.unwrap()
				.version,
			MinecraftVersionDeser::Latest(MinecraftLatestVersion::Release)
		);

		assert_eq!(
			serde_json::from_str::<Test>(r#"{"version": "latest_snapshot"}"#)
				.unwrap()
				.version,
			MinecraftVersionDeser::Latest(MinecraftLatestVersion::Snapshot)
		);
	}
}
