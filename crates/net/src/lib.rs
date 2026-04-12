//! Note: The asynchronous functions in this library expect the use of the Tokio runtime and may panic
//! if it is not used

use anyhow::Context;
use bytes::Bytes;
use reqwest::{Client, Url};

/// Interacting with the CurseForge API
pub mod curseforge;
/// Download utilities
pub mod download;
/// GitHub releases API
pub mod github;
/// Interacting with the Modrinth API
pub mod modrinth;
/// Downloading the NeoForge installer
pub mod neoforge;
/// Interacting with the Smithed API
pub mod smithed;

/// Loads bytes from a file path or URL
pub async fn load_from_uri(uri: &str, client: &Client) -> anyhow::Result<Bytes> {
	if let Ok(url) = Url::parse(uri) {
		download::bytes(url, client).await
	} else {
		std::fs::read(uri)
			.map(Bytes::from)
			.context("Failed to read from file")
	}
}
