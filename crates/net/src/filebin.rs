use anyhow::Context;
use reqwest::Client;

/// Uploads a file to filebin
pub async fn upload(
	contents: String,
	bin_id: &str,
	filename: &str,
	client: &Client,
) -> anyhow::Result<()> {
	client
		.post(format!("https://filebin.net/{bin_id}/{filename}"))
		.body(contents.into_bytes())
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}

/// Downloads a file from filebin.net
pub async fn download(bin_id: &str, filename: &str, client: &Client) -> anyhow::Result<String> {
	client
		.get(format!("https://filebin.net/{bin_id}/{filename}"))
		.header("Cookie", "verified=2025-05-24")
		.header("User-Agent", "curl/7.68.0")
		.send()
		.await?
		.error_for_status()?
		.text()
		.await
		.context("Failed to download")
}
