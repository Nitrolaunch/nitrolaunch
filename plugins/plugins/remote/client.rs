use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use nitro_core::io::{json_from_file, json_to_file};
use nitro_net::download::Client;
use nitrolaunch::config_crate::instance::InstanceConfig;
use nitrolaunch::config_crate::template::TemplateConfig;
use reqwest::Certificate;
use reqwest::tls::TlsInfo;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::OnceCell;

use crate::output::InputEvent;
use crate::server::GetJobResponse;
use crate::server::InputJobRequest;
use crate::server::PORT;
use crate::server::SyncResponse;

/// Client for interacting with a remote control server.
/// Mostly handles TLS / certificates.
#[derive(Clone)]
pub struct RemoteClient {
	settings: RemoteSettings,
	client: Arc<OnceCell<Client>>,
	remote_dir: PathBuf,
}

impl RemoteClient {
	pub fn new(settings: RemoteSettings, dir: &Path) -> Self {
		let remote_dir = get_remote_data_dir(dir, &settings.id);
		let _ = std::fs::create_dir_all(&remote_dir);
		Self {
			settings,
			remote_dir,
			client: Arc::new(OnceCell::new()),
		}
	}

	/// Creates the HTTP client if it doesn't exist by fetching the certificate from the remote server
	async fn client(&self) -> anyhow::Result<&Client> {
		if self.client.get().is_none() {
			let cert_path = self.remote_dir.join("cert.der");
			let cert_der = if cert_path.exists() {
				std::fs::read(&cert_path).context("Failed to read cached certificate")?
			} else {
				let cert_client = Client::builder()
					.use_rustls_tls()
					.danger_accept_invalid_certs(true)
					.tls_info(true)
					.https_only(true)
					.build()?;
				let url = format_url(&self.settings, "");
				let response = cert_client
					.get(url)
					.send()
					.await
					.context("Failed to send request")?
					.error_for_status()
					.context("Server reported an error")?;
				let cert_der = response
					.extensions()
					.get::<TlsInfo>()
					.and_then(TlsInfo::peer_certificate)
					.context("Server did not provide a certificate")?
					.to_vec();

				let _ = std::fs::write(&cert_path, &cert_der);
				cert_der
			};

			let cert_der =
				Certificate::from_der(&cert_der).context("Failed to read certificate")?;
			let client = Client::builder()
				.use_rustls_tls()
				.tls_built_in_root_certs(false)
				.add_root_certificate(cert_der)
				.danger_accept_invalid_hostnames(true)
				.https_only(true)
				.build()?;

			self.client.set(client).unwrap();
		}
		Ok(self.client.get().unwrap())
	}

	/// Fetches instances, templates, and the base template from the remote. If force is not specified, the cache will always be used.
	pub async fn sync(&self, force: bool) -> anyhow::Result<SyncResponse> {
		let path = get_sync_cache_path(&self.remote_dir);

		if !force && path.exists() {
			json_from_file(path)
		} else {
			let client = self.client().await?;
			let data: SyncResponse = download_json("sync", &self.settings, &client)
				.await
				.context("Failed to download remote config")?;
			let _ = json_to_file(path, &data);
			Ok(data)
		}
	}

	/// Fetches a job from the remote
	pub async fn get_job(&self, job_id: u64) -> anyhow::Result<Option<GetJobResponse>> {
		let client = self.client().await?;
		download_json_optional(&format!("jobs/{job_id}"), None, &self.settings, &client).await
	}

	/// Sends an input for a job on the remote
	pub async fn send_input(&self, job_id: u64, input: InputEvent) -> anyhow::Result<()> {
		let client = self.client().await?;
		let body = InputJobRequest { event: input };
		let body = serde_json::to_string(&body).context("Failed to serialize input")?;
		post(
			&format!("jobs/{job_id}/input"),
			Some(body),
			&self.settings,
			&client,
		)
		.await
	}

	/// Launches an instance on the remote server and returns the job ID
	pub async fn launch(&self, request: crate::server::LaunchRequest) -> anyhow::Result<u64> {
		let client = self.client().await?;
		let body = serde_json::to_string(&request).context("Failed to serialize launch request")?;
		download_job_number("launch", Some(body), &self.settings, &client).await
	}

	/// Updates an instance on the remote server and returns the job ID
	pub async fn update(&self, request: crate::server::UpdateRequest) -> anyhow::Result<u64> {
		let client = self.client().await?;
		let body = serde_json::to_string(&request).context("Failed to serialize update request")?;
		download_job_number("update", Some(body), &self.settings, &client).await
	}

	/// Configures an instance on the remote server
	pub async fn configure_instance(
		&self,
		dir: &Path,
		id: &str,
		instance_config: &InstanceConfig,
	) -> anyhow::Result<()> {
		let client = self.client().await?;
		let body = serde_json::to_string(instance_config)
			.context("Failed to serialize instance config")?;
		post(
			&format!("instances/{id}/configure"),
			Some(body),
			&self.settings,
			&client,
		)
		.await?;

		// Update cache
		let remote_dir = get_remote_data_dir(dir, &self.settings.id);
		let path = get_sync_cache_path(&remote_dir);
		if let Ok(mut data) = json_from_file::<SyncResponse>(path.clone()) {
			data.instances
				.insert(id.to_string(), instance_config.clone());
			let _ = json_to_file(path, &data);
		}

		Ok(())
	}

	/// Configures a template on the remote server
	pub async fn configure_template(
		&self,
		dir: &Path,
		id: &str,
		template_config: &TemplateConfig,
	) -> anyhow::Result<()> {
		let client = self.client().await?;
		let body = serde_json::to_string(template_config)
			.context("Failed to serialize template config")?;
		post(
			&format!("templates/{id}/configure"),
			Some(body),
			&self.settings,
			&client,
		)
		.await?;

		// Update cache
		let remote_dir = get_remote_data_dir(dir, &self.settings.id);
		let path = get_sync_cache_path(&remote_dir);
		if let Ok(mut data) = json_from_file::<SyncResponse>(path.clone()) {
			data.templates
				.insert(id.to_string(), template_config.clone());
			let _ = json_to_file(path, &data);
		}

		Ok(())
	}

	/// Configures the base template on the remote server
	pub async fn configure_base_template(
		&self,
		dir: &Path,
		template_config: &TemplateConfig,
	) -> anyhow::Result<()> {
		let client = self.client().await?;
		let body = serde_json::to_string(template_config)
			.context("Failed to serialize template config")?;
		post(
			"base_template/configure",
			Some(body),
			&self.settings,
			&client,
		)
		.await?;

		// Update cache
		let remote_dir = get_remote_data_dir(dir, &self.settings.id);
		let path = get_sync_cache_path(&remote_dir);
		if let Ok(mut data) = json_from_file::<SyncResponse>(path.clone()) {
			data.base_template = template_config.clone();
			let _ = json_to_file(path, &data);
		}

		Ok(())
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

fn get_sync_cache_path(remote_dir: &Path) -> PathBuf {
	remote_dir.join("remote_config.json")
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
	body: Option<String>,
	settings: &RemoteSettings,
	client: &Client,
) -> anyhow::Result<Option<T>> {
	let url = format_url(settings, subpath);
	let response = client
		.get(url)
		.body(body.unwrap_or_default())
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

async fn post(
	subpath: &str,
	body: Option<String>,
	settings: &RemoteSettings,
	client: &Client,
) -> anyhow::Result<()> {
	let url = format_url(settings, subpath);
	client
		.post(url)
		.body(body.unwrap_or_default())
		.header("Authorization", &settings.key)
		.send()
		.await
		.context("Failed to send request")?
		.error_for_status()
		.context("Server reported an error")?;

	Ok(())
}

fn format_url(settings: &RemoteSettings, subpath: &str) -> String {
	let address = format!("{}:{PORT}", settings.address);
	let mut url = format!("{address}/{}", subpath);
	if url.starts_with("http://") {
		url = url.replacen("http://", "https://", 1);
	} else if !url.starts_with("https://") {
		url = format!("https://{}", url);
	}
	url
}
