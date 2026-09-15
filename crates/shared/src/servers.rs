use serde::{Deserialize, Serialize};

use crate::loaders::Loader;

/// Information about a Minecraft server
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
