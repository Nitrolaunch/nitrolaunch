use serde::{Deserialize, Serialize};

use crate::loaders::Loader;

/// Information about a Minecraft server
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ServerInfo {
	/// The display name of the server
	pub name: String,
	/// The server address
	pub address: String,
	/// The server description
	pub description: String,
	/// Categories that the server belongs to
	pub categories: Vec<ServerCategory>,
	/// Icon of the server
	pub icon: Option<String>,
	/// Images of the server
	pub gallery: Vec<String>,
	/// The number of players currently online on the server
	pub online_players: Option<u32>,
	/// Minecraft versions this server supports
	pub minecraft_versions: Vec<String>,
	/// Loaders required to join the server
	pub loaders: Vec<Loader>,
	/// Whether the address is actually an application link to join the server
	pub is_application: bool,
}

/// The category of a server
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ServerCategory {
	Pvp,
	Pve,
	Survival,
	Creative,
	Minigames,
	Modded,
	Economy,
	Skyblock,
	Anarchy,
	Other,
}

/// Search parameters for finding servers
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ServerSearchParams {
	/// The number of packages to return
	pub count: u8,
	/// How many results to skip
	pub skip: usize,
	/// Textual search for server properties
	pub search: Option<String>,
	/// The Minecraft version to search for
	pub minecraft_version: Option<String>,
	/// Server categories to search for
	pub categories: Vec<ServerCategory>,
}

/// Results from searching for servers
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ServerSearchResults {
	/// The servers that match the search criteria
	pub results: Vec<ServerInfo>,
	/// The total number of results that match the search criteria
	pub total_results: usize,
}
