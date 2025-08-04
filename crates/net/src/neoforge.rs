use std::path::Path;

use reqwest::Client;
use serde::Deserialize;

use crate::download;

/// Base URL for installer versions
pub static VERSIONS_URL: &str =
	"https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

/// The list of versions
#[derive(Deserialize)]
pub struct Versions {
	/// The versions
	pub versions: Vec<String>,
}

/// Gets the list of available NeoForge versions
pub async fn get_versions(client: &Client) -> anyhow::Result<Vec<String>> {
	let versions: Versions = download::json(VERSIONS_URL, client).await?;

	Ok(versions.versions)
}

/// Gets the newest NeoForge version from a list of versions
pub fn get_latest_neoforge_version<'a>(
	versions: &'a [String],
	minecraft_version: &str,
) -> Option<&'a String> {
	versions
		.iter()
		.rev()
		.find(|neoforge_version| is_version_compatible(neoforge_version, minecraft_version))
}

/// Checks if a NeoForge version is made for a given Minecraft version
pub fn is_version_compatible(neoforge_version: &str, minecraft_version: &str) -> bool {
	// Get the major version and patch of a release (remove the 1.)
	let minecraft_major_version = &minecraft_version[2..];
	neoforge_version.starts_with(minecraft_major_version)
}

/// Downloads the installer for the given NeoForge version
pub async fn download_installer(
	neoforge_version: &str,
	path: &Path,
	client: &Client,
) -> anyhow::Result<()> {
	let url =
		format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar");

	download::file(&url, path, client).await
}
