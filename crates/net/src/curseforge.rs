use anyhow::Context;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::download::user_agent;

/// Requests a sub-url from the CurseForge API
pub async fn request_api<D: DeserializeOwned>(
	url_path: &str,
	api_key: &str,
	client: &Client,
) -> anyhow::Result<D> {
	let resp = client
		.get(String::from("https://api.curseforge.com/") + url_path)
		.header("User-Agent", user_agent())
		.header("x-api-key", api_key)
		.send()
		.await
		.context("Failed to send request")?
		.error_for_status()
		.context("Server reported an error")?;

	Ok(resp.error_for_status()?.json().await?)
}

/// Requests a sub-url from the CurseForge API for text
pub async fn request_api_raw(
	url_path: &str,
	api_key: &str,
	client: &Client,
) -> anyhow::Result<String> {
	let resp = client
		.get(String::from("https://api.curseforge.com/") + url_path)
		.header("x-api-key", api_key)
		.header("User-Agent", user_agent())
		.send()
		.await
		.context("Failed to send request")?
		.error_for_status()
		.context("Server reported an error")?;

	Ok(resp.error_for_status()?.text().await?)
}

/// Gets a CurseForge mod with the given ID from the API
pub async fn get_mod(id: &str, api_key: &str, client: &Client) -> anyhow::Result<CurseMod> {
	let mut response: CurseModResponse =
		request_api(&format!("v1/mods/{id}"), api_key, client).await?;
	Ok(response.data.remove(0))
}

/// Gets a CurseForge mod with the given ID from the API
pub async fn get_mod_raw(id: &str, api_key: &str, client: &Client) -> anyhow::Result<String> {
	request_api_raw(&format!("v1/mods/{id}"), api_key, client).await
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseModResponse {
	pub data: Vec<CurseMod>,
}

/// A project on CurseForge
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseMod {
	/// Unique ID of the mod
	pub id: u32,
	/// Game ID for the mod
	pub game_id: u32,
	/// Display name for the mod
	pub name: String,
	/// Unique slug for the mod
	pub slug: String,
	/// Short description of the mod
	pub summary: String,
	/// How many downloads the mod has
	pub download_count: u32,
}
