use std::path::PathBuf;

use anyhow::{Context, bail};
use nitro_plugin::{
	api::{
		utils::SearchCache,
		wasm::{WASMPlugin, sys::get_data_dir},
	},
	nitro_wasm_plugin,
};
use nitro_shared::servers::{ServerInfo, ServerSearchParams, ServerSearchResults};
use serde::Deserialize;
use wstd::{
	http::{Client, Request, StatusCode},
	runtime::block_on,
};

const API_URL: &str = "https://api.breakblocks.com/api/v0.1/servers/find";

nitro_wasm_plugin!(main, "breakblocks");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.search_server_repository(|arg| {
		if arg.repository.as_deref() != Some("breakblocks") {
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
		SearchCache::open(&cache_path, 300).context("Failed to open BreakBlocks search cache")?;
	if let Some(results) = search_cache.check::<_, ServerSearchResults>(&parameters) {
		return Ok(results);
	}

	let results = fetch_servers(&parameters).await?;
	let _ = search_cache.write(&parameters, &results);
	Ok(results)
}

async fn fetch_servers(parameters: &ServerSearchParams) -> anyhow::Result<ServerSearchResults> {
	let count = parameters.count.clamp(1, 20);
	let page = parameters.skip / count as usize + 1;
	let mut query = vec![
		format!("page={page}"),
		format!("limit={count}"),
		"favicon=true".into(),
	];

	if let Some(version) = &parameters.minecraft_version {
		query.push(format!("version={}", encode_query(version)));
	}
	if let Some(search) = &parameters.search {
		query.push(format!("motd={}", encode_query(&format!("*{search}*"))));
	}

	let request = Request::get(format!("{API_URL}?{}", query.join("&"))).body("")?;
	let mut response = Client::new()
		.send(request)
		.await
		.context("Failed to fetch BreakBlocks servers")?;
	if response.status() != StatusCode::OK {
		bail!(
			"Failed to fetch BreakBlocks servers: HTTP {}",
			response.status()
		);
	}

	let response: BreakBlocksResponse = response
		.body_mut()
		.json()
		.await
		.context("Failed to parse BreakBlocks server response")?;

	Ok(ServerSearchResults {
		results: response.results.into_iter().map(ServerInfo::from).collect(),
		total_results: response.filtered,
	})
}

fn cache_path() -> PathBuf {
	get_data_dir().join("internal/breakblocks/search_cache.json")
}

fn encode_query(value: &str) -> String {
	let mut encoded = String::new();
	for byte in value.bytes() {
		if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'*') {
			encoded.push(byte as char);
		} else {
			encoded.push_str(&format!("%{byte:02X}"));
		}
	}
	encoded
}

#[derive(Deserialize)]
struct BreakBlocksResponse {
	filtered: usize,
	results: Vec<BreakBlocksServer>,
}

#[derive(Deserialize)]
struct BreakBlocksServer {
	address: String,
	port: u16,
	version: Option<String>,
	players_online: Option<u32>,
	motd: Option<String>,
	motd_stripped: Option<String>,
	favicon: Option<String>,
}

impl From<BreakBlocksServer> for ServerInfo {
	fn from(server: BreakBlocksServer) -> Self {
		let name = server
			.motd_stripped
			.clone()
			.filter(|motd| !motd.is_empty())
			.unwrap_or_else(|| server.address.clone());
		let description = server.motd_stripped.or(server.motd).unwrap_or_default();
		let address = if server.port == 25565 {
			server.address
		} else {
			format!("{}:{}", server.address, server.port)
		};

		ServerInfo {
			name,
			address,
			description,
			icon: server.favicon,
			online_players: server.players_online,
			minecraft_versions: server.version.into_iter().collect(),
			..Default::default()
		}
	}
}
