use crate::download::{self, user_agent};
use anyhow::Context;
use nitro_shared::pkg::{PackageCategory, PackageSearchParameters};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

/// Get a Smithed pack from the API
pub async fn get_pack(id: &str, client: &Client) -> anyhow::Result<Pack> {
	let url = format!("{API_URL}/packs/{id}");
	download::json(url, client).await
}

/// Get a Smithed pack from the API, returning None on 404
pub async fn get_pack_optional(id: &str, client: &Client) -> anyhow::Result<Option<Pack>> {
	let url = format!("{API_URL}/packs/{id}");

	let resp = client
		.get(url)
		.header("User-Agent", user_agent())
		.send()
		.await
		.context("Failed to send request")?;
	if resp.status() == StatusCode::NOT_FOUND {
		return Ok(None);
	}

	let resp = resp
		.error_for_status()
		.context("Server returned an error")?;

	resp.json()
		.await
		.map(Some)
		.context("Failed to deserialize JSON")
}

/// API URL
const API_URL: &str = "https://api.smithed.dev/v2";

/// A Smithed pack
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pack {
	pub id: String,
	pub display: PackDisplay,
	pub versions: Vec<PackVersion>,
	#[serde(default)]
	pub categories: Vec<String>,
}

/// Display info for a Smithed pack
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackDisplay {
	pub name: String,
	pub description: String,
	pub icon: String,
	pub hidden: bool,
	#[serde(default)]
	pub web_page: Option<String>,
	#[serde(default)]
	pub gallery: Vec<GalleryEntry>,
}

/// Etnry in a Smithed pack gallery
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GalleryEntry {}

/// Version of a pack
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackVersion {
	pub name: String,
	pub downloads: PackDownloads,
	pub supports: Vec<String>,
	#[serde(default)]
	pub dependencies: Vec<PackReference>,
}

/// Downloads for a pack version
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackDownloads {
	pub datapack: Option<String>,
	pub resourcepack: Option<String>,
}

/// Reference to a pack version
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackReference {
	pub id: String,
	pub version: String,
}

/// Get a Smithed bundle from the API
pub async fn get_bundle(id: &str, client: &Client) -> anyhow::Result<Bundle> {
	let url = format!("{API_URL}/bundles/{id}");
	download::json(url, client).await
}

/// A Smithed bundle
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bundle {
	pub id: String,
	pub versions: Vec<BundleVersion>,
}

/// Version of a bundle
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BundleVersion {
	pub name: String,
	pub supports: Vec<String>,
	pub packs: Vec<PackReference>,
}

/// Search packs from the Smithed API
pub async fn search_packs(
	params: PackageSearchParameters,
	client: &Client,
) -> anyhow::Result<Vec<PackSearchResult>> {
	let limit = if params.count > 100 {
		100
	} else {
		params.count
	};
	let search = if let Some(search) = params.search {
		format!("&search={search}")
	} else {
		String::new()
	};
	let page = params.skip / params.count as usize + 1;

	let filters = create_search_filters(params.minecraft_versions, params.categories);

	let url = format!(
		"{API_URL}/packs?limit={limit}{search}&page={page}{filters}&scope=data&scope=meta.rawId"
	);

	download::json(url, client).await
}

/// A single pack search result
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackSearchResult {
	pub id: String,
	#[serde(rename = "displayName")]
	pub display_name: String,
	pub data: Pack,
	pub meta: PackSearchResultMeta,
}

/// Metadata for a pack search result
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackSearchResultMeta {
	/// The plaintext ID / slug of this pack
	#[serde(rename = "rawId")]
	pub raw_id: String,
}

/// Count packs from the Smithed API that match a criteris
pub async fn count_packs(
	params: PackageSearchParameters,
	client: &Client,
) -> anyhow::Result<usize> {
	let search = if let Some(search) = params.search {
		format!("search={search}")
	} else {
		String::new()
	};

	let filters = create_search_filters(params.minecraft_versions, params.categories);

	let url = format!("{API_URL}/packs/count?{search}{filters}");

	download::json(url, client).await
}

fn create_search_filters(
	minecraft_versions: Vec<String>,
	categories: Vec<PackageCategory>,
) -> String {
	let versions = minecraft_versions
		.into_iter()
		.map(|x| format!("&version={x}"))
		.collect::<Vec<_>>()
		.join("");

	let categories = categories
		.into_iter()
		.filter_map(|x| convert_category(x))
		.map(|x| format!("&category={x}"))
		.collect::<Vec<_>>()
		.join("");

	format!("{versions}{categories}")
}

/// Get the URL to a Smithed pack gallery entry
pub fn get_gallery_url(pack_id: &str, index: u8) -> String {
	format!("https://api.smithed.dev/v2/packs/{pack_id}/gallery/{index}")
}

fn convert_category(category: PackageCategory) -> Option<&'static str> {
	match category {
		PackageCategory::Extensive => Some("Extensive"),
		PackageCategory::Lightweight => Some("Lightweight"),
		PackageCategory::Tweaks => Some("QoL"),
		PackageCategory::VanillaPlus => Some("Vanilla+"),
		PackageCategory::Technology => Some("Tech"),
		PackageCategory::Magic => Some("Magic"),
		PackageCategory::Exploration => Some("Exploration"),
		PackageCategory::Worldgen => Some("World Overhaul"),
		PackageCategory::Library => Some("Library"),
		_ => None,
	}
}
