use std::path::PathBuf;

use anyhow::{Context, bail};
use nitro_plugin::{
	api::{
		utils::SearchCache,
		wasm::{WASMPlugin, sys::get_data_dir},
	},
	nitro_wasm_plugin,
};
use nitro_shared::servers::{ServerCategory, ServerInfo, ServerSearchParams, ServerSearchResults};
use serde::Deserialize;
use wstd::{
	http::{Client, Request, StatusCode},
	runtime::block_on,
};

const REPOSITORY_ID: &str = "anyserver";
const API_URL: &str = "https://anyserver.pro/api/servers";

nitro_wasm_plugin!(main, "anyserver");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.search_server_repository(|arg| {
		if arg.repository.as_deref() != Some(REPOSITORY_ID) {
			return Ok(ServerSearchResults::default());
		}

		block_on(search_servers(arg))
	})?;

	Ok(())
}

async fn search_servers(parameters: ServerSearchParams) -> anyhow::Result<ServerSearchResults> {
	let cache_path = cache_path();
	if let Some(parent) = cache_path.parent() {
		std::fs::create_dir_all(parent).context("Failed to create search cache directory")?;
	}

	let mut search_cache =
		SearchCache::open(&cache_path, 300).context("Failed to open AnyServer search cache")?;
	if let Some(results) = search_cache.check::<_, ServerSearchResults>(&parameters) {
		return Ok(results);
	}

	let results = fetch_servers(&parameters).await?;
	let _ = search_cache.write(&parameters, &results);
	Ok(results)
}

async fn fetch_servers(parameters: &ServerSearchParams) -> anyhow::Result<ServerSearchResults> {
	let count = parameters.count.max(1).min(255);
	let mut query = vec![
		"game=mc_java".into(),
		format!("limit={count}"),
		format!("offset={}", parameters.skip),
		"hide_offline=true".into(),
	];

	if let Some(version) = &parameters.minecraft_version {
		query.push(format!("version={}", encode_query(version)));
	}
	if !parameters.categories.is_empty() {
		let categories = parameters
			.categories
			.iter()
			.map(category_query_name)
			.collect::<Vec<_>>()
			.join(",");
		query.push(format!("tags={}", encode_query(&categories)));
	}
	if let Some(search) = &parameters.search {
		query.push(format!("search={}", encode_query(search)));
	}

	let request = Request::get(format!("{API_URL}?{}", query.join("&"))).body("")?;
	let mut response = Client::new()
		.send(request)
		.await
		.context("Failed to fetch AnyServer servers")?;
	if response.status() != StatusCode::OK {
		bail!(
			"Failed to fetch AnyServer servers: HTTP {}",
			response.status()
		);
	}

	let response: AnyServerResponse = response
		.body_mut()
		.json()
		.await
		.context("Failed to parse AnyServer server response")?;

	Ok(ServerSearchResults {
		results: response
			.servers
			.into_iter()
			.filter_map(AnyServer::into_server_info)
			.collect(),
		total_results: response.total,
	})
}

fn cache_path() -> PathBuf {
	get_data_dir().join("internal/anyserver/search_cache.json")
}

fn encode_query(value: &str) -> String {
	let mut encoded = String::new();
	for byte in value.bytes() {
		if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
			encoded.push(byte as char);
		} else {
			encoded.push_str(&format!("%{byte:02X}"));
		}
	}
	encoded
}

fn category_query_name(category: &ServerCategory) -> &'static str {
	match category {
		ServerCategory::Pvp => "pvp",
		ServerCategory::Pve => "pve",
		ServerCategory::Survival => "survival",
		ServerCategory::Creative => "creative",
		ServerCategory::Minigames => "minigames",
		ServerCategory::Modded => "modded",
		ServerCategory::Economy => "economy",
		ServerCategory::Skyblock => "skyblock",
		ServerCategory::Anarchy => "anarchy",
		ServerCategory::Other => "other",
	}
}

#[derive(Deserialize)]
struct AnyServerResponse {
	total: usize,
	servers: Vec<AnyServer>,
}

#[derive(Deserialize)]
struct AnyServer {
	name: String,
	address: Option<String>,
	port: Option<u16>,
	version: Option<String>,
	category: Option<String>,
	thumbnail: Option<String>,
	server_description: Option<String>,
	server_summary: Option<String>,
	is_online: Option<u8>,
	player_count: Option<u32>,
}

impl AnyServer {
	fn into_server_info(self) -> Option<ServerInfo> {
		let address = self.address?;
		let port = self.port.unwrap_or(25565);
		let address = if port == 25565 {
			address
		} else {
			format!("{address}:{port}")
		};
		let description = self
			.server_description
			.or(self.server_summary)
			.unwrap_or_default();
		let categories = self
			.category
			.unwrap_or_default()
			.split(',')
			.filter_map(|category| parse_category(category.trim()))
			.collect();
		let minecraft_versions = self
			.version
			.unwrap_or_default()
			.split(',')
			.map(str::trim)
			.filter(|version| !version.is_empty())
			.map(str::to_owned)
			.collect();

		Some(ServerInfo {
			name: self.name,
			address,
			description,
			categories,
			icon: self.thumbnail.map(|thumbnail| {
				if thumbnail.starts_with("http://") || thumbnail.starts_with("https://") {
					thumbnail
				} else {
					format!("https://cdn.anyserver.pro/thumbnails/{thumbnail}")
				}
			}),
			online_players: self
				.is_online
				.filter(|online| *online != 0)
				.and(self.player_count),
			minecraft_versions,
			..Default::default()
		})
	}
}

fn parse_category(category: &str) -> Option<ServerCategory> {
	Some(match category.to_ascii_lowercase().as_str() {
		"pvp" => ServerCategory::Pvp,
		"pve" => ServerCategory::Pve,
		"survival" => ServerCategory::Survival,
		"creative" => ServerCategory::Creative,
		"minigames" | "mini-games" | "mini games" => ServerCategory::Minigames,
		"modded" => ServerCategory::Modded,
		"economy" => ServerCategory::Economy,
		"skyblock" => ServerCategory::Skyblock,
		"anarchy" => ServerCategory::Anarchy,
		"other" => ServerCategory::Other,
		_ => return None,
	})
}
