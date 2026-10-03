use std::path::{Path, PathBuf};

use anyhow::Context;
use nitro_core::io::{json_from_file, json_to_file};
use nitro_net::download::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::server::GetJobResponse;
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

/// Fetches a job from the remote
pub async fn get_job(
	settings: &RemoteSettings,
	client: &Client,
	job_id: u64,
) -> anyhow::Result<Option<GetJobResponse>> {
	download_json_optional(&format!("jobs/{job_id}"), settings, client).await
}

/// Launches an instance on the remote server and returns the job ID
pub async fn launch(
	settings: &RemoteSettings,
	client: &Client,
	request: crate::server::LaunchRequest,
) -> anyhow::Result<u64> {
	let body = serde_json::to_string(&request).context("Failed to serialize launch request")?;
	download_job_number("launch", Some(body), settings, client).await
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
	let url = format_url(settings, subpath);
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

async fn download_json_optional<T: DeserializeOwned>(
	subpath: &str,
	settings: &RemoteSettings,
	client: &Client,
) -> anyhow::Result<Option<T>> {
	let url = format_url(settings, subpath);
	let response = client
		.get(url)
		.header("Authorization", &settings.key)
		.send()
		.await
		.context("Failed to send request")?;

	if response.status().is_success() {
		let data = response.json().await.context("Failed to parse JSON")?;
		Ok(Some(data))
	} else if response.status().as_u16() == 404 {
		Ok(None)
	} else {
		Err(anyhow::anyhow!(
			"Server reported an error: {}",
			response.status()
		))
	}
}

async fn download_job_number(
	subpath: &str,
	body: Option<String>,
	settings: &RemoteSettings,
	client: &Client,
) -> anyhow::Result<u64> {
	let url = format_url(settings, subpath);
	let response = client
		.post(url)
		.body(body.unwrap_or_default())
		.header("Authorization", &settings.key)
		.send()
		.await
		.context("Failed to send request")?
		.error_for_status()
		.context("Server reported an error")?;

	let text = response.text().await.context("Failed to read response")?;
	let number: u64 = text.trim().parse().context("Failed to parse number")?;
	Ok(number)
}

fn format_url(settings: &RemoteSettings, subpath: &str) -> String {
	let address = format!("{}:{PORT}", settings.address);
	let mut url = format!("{address}/{}", subpath);
	if !url.starts_with("http://") && !url.starts_with("https://") {
		url = format!("http://{}", url);
	}
	url
}
