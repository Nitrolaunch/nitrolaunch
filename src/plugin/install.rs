use std::{
	collections::HashMap,
	env::consts::{ARCH, OS},
	io::Cursor,
};

use anyhow::{bail, Context};
use nitro_core::{io::json_from_file, net::download};
use nitro_net::github::{get_github_releases, GithubAsset};
use nitro_plugin::plugin::{PluginManifest, PluginMetadata};
use nitro_shared::{
	output::{MessageContents, MessageLevel, NitroOutput},
	util::TARGET_BITS_STR,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::io::paths::Paths;

use super::PluginManager;

/// Information about a single verified plugin
#[derive(Serialize, Deserialize)]
pub struct VerifiedPlugin {
	/// The ID of the plugin
	pub id: String,
	/// The current version of the plugin
	pub version: Option<String>,
	/// Metadata for the plugin
	#[serde(flatten)]
	pub meta: PluginMetadata,
	/// The organization / user that owns the repo where this plugin is
	pub github_owner: String,
	/// The name of the GitHub repo where this plugin is
	pub github_repo: String,
}

/// Gets the verified plugin list
pub async fn get_verified_plugins(
	client: &Client,
	offline: bool,
) -> anyhow::Result<HashMap<String, VerifiedPlugin>> {
	let mut list: HashMap<String, VerifiedPlugin> =
		serde_json::from_str(include_str!("verified_plugins.json"))
			.context("Failed to deserialize core verified list")?;

	if !offline {
		if let Ok(remote_list) = download::json::<HashMap<String, VerifiedPlugin>>(
			"https://github.com/Nitrolaunch/nitrolaunch/blob/main/src/plugin/verified_plugins.json",
			client,
		)
		.await
		{
			list.extend(remote_list);
		}
	}

	Ok(list)
}

impl VerifiedPlugin {
	/// Gets the list of candidate GitHub assets for this plugin, ordered from newest to oldest
	pub async fn get_candidate_assets(
		&self,
		version: Option<&str>,
		client: &Client,
	) -> anyhow::Result<Vec<CandidateAsset>> {
		// Get releases
		let releases = get_github_releases(&self.github_owner, &self.github_repo, client)
			.await
			.context("Failed to get GitHub releases")?;

		let mut assets = Vec::new();

		for release in releases {
			// Only get releases that are tagged correctly
			if !release.tag_name.contains("plugin") {
				continue;
			}

			let Some(release_version) = extract_release_plugin_version(&release.tag_name) else {
				continue;
			};

			// Check the plugin version
			if let Some(requested_version) = &version {
				if requested_version != &release_version {
					continue;
				}
			}

			// Select the correct asset
			for asset in release.assets {
				if !asset.name.contains(&self.id) {
					continue;
				}

				// Check the system
				if !asset.name.contains("universal") && !asset.name.contains(OS) {
					continue;
				}

				if (asset.name.contains("x86")
					|| asset.name.contains("arm")
					|| asset.name.contains("aarch64"))
					&& !asset.name.contains(ARCH)
				{
					continue;
				}

				if (asset.name.contains("32bit") || asset.name.contains("64bit"))
					&& !asset.name.contains(TARGET_BITS_STR)
				{
					continue;
				}

				assets.push(CandidateAsset {
					asset,
					version: release_version.to_string(),
				});
			}
		}

		Ok(assets)
	}

	/// Install or update this plugin
	pub async fn install(
		&self,
		version: Option<&str>,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let assets = self.get_candidate_assets(version, client).await?;

		let Some(asset) = assets.first() else {
			bail!("Could not find a release that matches your system");
		};

		// Actually download and install
		if !asset.asset.content_type.contains("zip") {
			bail!("Plugin asset is not a ZIP file");
		}
		let zip = download::bytes(&asset.asset.browser_download_url, client)
			.await
			.context("Failed to download zipped plugin")?;

		let mut zip = ZipArchive::new(Cursor::new(zip)).context("Failed to read zip archive")?;

		PluginManager::remove_plugin(&self.id, paths)
			.context("Failed to remove existing plugin")?;
		let dir = paths.plugins.join(&self.id);
		std::fs::create_dir_all(&dir).context("Failed to create plugin directory")?;

		zip.extract(&dir)
			.context("Failed to extract plugin files")?;

		let manifest: PluginManifest =
			json_from_file(dir.join("plugin.json")).context("Failed to read plugin manifest")?;

		if let Some(install_message) = manifest.install_message {
			o.display(
				MessageContents::Warning(install_message),
				MessageLevel::Important,
			);
		}

		let _ = PluginManager::enable_plugin(&self.id, paths);

		Ok(())
	}
}

/// Asset for a plugin that matches the system and version requirements
pub struct CandidateAsset {
	/// The asset
	pub asset: GithubAsset,
	/// The version name of the release this asset is from
	pub version: String,
}

/// Splits the parts of a release name to extract the plugin version
pub fn extract_release_plugin_version(tag_name: &str) -> Option<&str> {
	// Format is plugin-<id>-<version>
	let mut tag_parts = tag_name.split('-');
	tag_parts.nth(2)
}
