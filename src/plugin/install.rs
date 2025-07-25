use std::{
	collections::HashMap,
	env::consts::{ARCH, OS},
	io::Cursor,
};

use anyhow::{bail, Context};
use nitro_core::{io::json_from_file, net::download};
use nitro_net::github::get_github_releases;
use nitro_plugin::plugin::PluginManifest;
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
	/// The display name of the plugin
	pub name: Option<String>,
	/// The organization / user that owns the repo where this plugin is
	pub github_owner: String,
	/// The name of the GitHub repo where this plugin is
	pub github_repo: String,
	/// Short description of the plugin
	pub description: String,
}

/// Gets the verified plugin list
pub async fn get_verified_plugins(
	client: &Client,
) -> anyhow::Result<HashMap<String, VerifiedPlugin>> {
	let mut list: HashMap<String, VerifiedPlugin> =
		serde_json::from_str(include_str!("verified_plugins.json"))
			.context("Failed to deserialize core verified list")?;

	if let Ok(remote_list) = download::json::<HashMap<String, VerifiedPlugin>>(
		"https://github.com/Nitrolaunch/nitrolaunch/blob/main/src/plugin/verified_plugins.json",
		client,
	)
	.await
	{
		list.extend(remote_list);
	}

	Ok(list)
}

impl VerifiedPlugin {
	/// Install or update this plugin
	pub async fn install(
		&self,
		version: Option<&str>,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		// Get releases
		let releases = get_github_releases(&self.github_owner, &self.github_repo, client)
			.await
			.context("Failed to get GitHub releases")?;

		let mut selected_asset = None;
		'outer: for release in releases {
			// Only get releases that are tagged correctly
			if !release.tag_name.contains("plugin") {
				continue;
			}

			// Grab the version from the tag, skipping past the 'plugin' and the id
			let mut tag_parts = release.tag_name.split('-');
			tag_parts.next();
			tag_parts.next();
			let Some(release_version) = tag_parts.next() else {
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

				if asset.name.contains("x86")
					|| asset.name.contains("arm")
					|| asset.name.contains("aarch64")
				{
					if !asset.name.contains(ARCH) {
						continue;
					}
				}

				if asset.name.contains("32bit") || asset.name.contains("64bit") {
					if !asset.name.contains(TARGET_BITS_STR) {
						continue;
					}
				}

				selected_asset = Some(asset);
				break 'outer;
			}
		}

		let Some(asset) = selected_asset else {
			bail!("Could not find a release that matches your system");
		};

		// Actually download and install
		let manifest = if asset.content_type.contains("zip") {
			let zip = download::bytes(asset.browser_download_url, client)
				.await
				.context("Failed to download zipped plugin")?;

			let mut zip =
				ZipArchive::new(Cursor::new(zip)).context("Failed to read zip archive")?;

			PluginManager::remove_plugin(&self.id, paths)
				.context("Failed to remove existing plugin")?;
			let dir = paths.plugins.join(&self.id);
			std::fs::create_dir_all(&dir).context("Failed to create plugin directory")?;

			zip.extract(&dir)
				.context("Failed to extract plugin files")?;

			json_from_file::<PluginManifest>(dir.join("plugin.json"))
				.context("Failed to read plugin manifest")?
		} else if asset.content_type.contains("json") {
			let bytes = download::bytes(asset.browser_download_url, client)
				.await
				.context("Failed to download plugin JSON")?;

			PluginManager::uninstall_plugin(&self.id, paths)
				.context("Failed to remove existing plugin")?;
			let path = paths.plugins.join(format!("{}.json", &self.id));
			std::fs::write(path, &bytes).context("Failed to write plugin JSON")?;

			serde_json::from_slice(&bytes).context("Failed to read plugin manifest")?
		} else {
			bail!("Plugin files are not of the correct type");
		};

		if let Some(install_message) = manifest.install_message {
			o.display(
				MessageContents::Warning(install_message),
				MessageLevel::Important,
			);
		}

		Ok(())
	}
}
