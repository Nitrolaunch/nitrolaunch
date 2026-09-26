use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_core::io::{json_from_file, json_to_file};
use nitro_net::download::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::server::PORT;
use crate::server::SyncResponse;

/// Fetches instances, templates, and the base template from the remote. If force is not specified, the cache will always be used.
pub async fn sync(
	settings: &RemoteSettings,
	client: &Client,
	dir: &Path,
	force: bool,
) -> anyhow::Result<SyncResponse> {
	let remote_dir = get_remote_data_dir(dir, &settings.id);
	let _ = std::fs::create_dir_all(&remote_dir);
	let path = remote_dir.join("remote_config.json");

	if !force && path.exists() {
		json_from_file(path)
	} else {
		let data: SyncResponse = download_json("sync", settings, client)
			.await
			.context("Failed to download remote config")?;
		let _ = json_to_file(path, &data);
		Ok(data)
	}
}

/// Settings on the client for a single remote server
#[derive(Serialize, Deserialize, Clone)]
pub struct RemoteSettings {
	pub id: String,
	pub address: String,
	pub key: String,
}

fn get_remote_data_dir(dir: &Path, remote_id: &str) -> PathBuf {
	dir.join("remote").join(remote_id)
}

async fn download_json<T: DeserializeOwned>(
	subpath: &str,
	settings: &RemoteSettings,
	client: &Client,
) -> anyhow::Result<T> {
	let address = format!("{}:{PORT}", settings.address);
	let mut url = format!("{address}/{}", subpath);
	if !url.starts_with("http://") && !url.starts_with("https://") {
		url = format!("http://{}", url);
	}
	client
		.get(url)
		.header("Authorization", &settings.key)
		.send()
		.await
		.context("Failed to send request")?
		.error_for_status()
		.context("Server reported an error")?
		.json()
		.await
		.context("Failed to parse JSON")
}
