use std::fmt::Display;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A modification applied to a client or server, such as a modloader or plugin loader
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Loader {
	/// No loader, just the default game
	#[default]
	Vanilla,
	/// MinecraftForge
	Forge,
	/// NeoForged
	NeoForged,
	/// Fabric Loader
	Fabric,
	/// Quilt Loader
	Quilt,
	/// LiteLoader
	LiteLoader,
	/// Risugami's Modloader
	Risugamis,
	/// Rift
	Rift,
	/// Paper server
	Paper,
	/// SpongeVanilla
	Sponge,
	/// SpongeForge
	SpongeForge,
	/// CraftBukkit
	CraftBukkit,
	/// Spigot
	Spigot,
	/// Glowstone
	Glowstone,
	/// Pufferfish
	Pufferfish,
	/// Purpur
	Purpur,
	/// Folia
	Folia,
	/// An unknown loader
	#[cfg_attr(not(feature = "schema"), serde(untagged))]
	Unknown(String),
}

impl Loader {
	/// Parses a Loader from a string
	pub fn parse_from_str(string: &str) -> Self {
		match string {
			"vanilla" => Self::Vanilla,
			"forge" => Self::Forge,
			"neoforged" => Self::NeoForged,
			"fabric" => Self::Fabric,
			"quilt" => Self::Quilt,
			"liteloader" => Self::LiteLoader,
			"risugamis" => Self::Risugamis,
			"rift" => Self::Rift,
			"paper" => Self::Paper,
			"sponge" => Self::Sponge,
			"craftbukkit" => Self::CraftBukkit,
			"spigot" => Self::Spigot,
			"glowstone" => Self::Glowstone,
			"pufferfish" => Self::Pufferfish,
			"purpur" => Self::Purpur,
			"folia" => Self::Folia,
			other => Self::Unknown(other.to_string()),
		}
	}
}

impl Display for Loader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Vanilla => write!(f, "Vanilla"),
			Self::Forge => write!(f, "Forge"),
			Self::NeoForged => write!(f, "NeoForged"),
			Self::Fabric => write!(f, "Fabric"),
			Self::Quilt => write!(f, "Quilt"),
			Self::LiteLoader => write!(f, "LiteLoader"),
			Self::Risugamis => write!(f, "Risugami's"),
			Self::Rift => write!(f, "Rift"),
			Self::Paper => write!(f, "Paper"),
			Self::Sponge => write!(f, "Sponge"),
			Self::SpongeForge => write!(f, "SpongeForge"),
			Self::CraftBukkit => write!(f, "CraftBukkit"),
			Self::Spigot => write!(f, "Spigot"),
			Self::Glowstone => write!(f, "Glowstone"),
			Self::Pufferfish => write!(f, "Pufferfish"),
			Self::Purpur => write!(f, "Purpur"),
			Self::Folia => write!(f, "Folia"),
			Self::Unknown(other) => write!(f, "{other}"),
		}
	}
}

/// Matcher for different types of loaders
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LoaderMatch {
	/// Matches any loader that supports loading Fabric mods
	FabricLike,
	/// Matches any loader that supports loading Forge mods
	ForgeLike,
	/// Matches any loader that can load Bukkit plugins
	Bukkit,
	/// Matches a specific loader
	#[serde(untagged)]
	Loader(Loader),
}

impl LoaderMatch {
	/// Parse a LoaderMatch from a string
	pub fn parse_from_str(string: &str) -> Self {
		match string {
			"fabriclike" => Self::FabricLike,
			"forgelike" => Self::ForgeLike,
			"bukkit" => Self::Bukkit,
			other => LoaderMatch::Loader(Loader::parse_from_str(other)),
		}
	}

	/// Checks if a loader matches
	pub fn matches(&self, other: &Loader) -> bool {
		match self {
			Self::FabricLike => matches!(other, Loader::Fabric | Loader::Quilt),
			Self::ForgeLike => matches!(
				other,
				Loader::Forge | Loader::NeoForged | Loader::SpongeForge
			),
			Self::Bukkit => matches!(
				other,
				Loader::Paper
					| Loader::CraftBukkit
					| Loader::Spigot
					| Loader::Glowstone
					| Loader::Pufferfish
					| Loader::Purpur
			),
			Self::Loader(loader) => loader == other,
		}
	}
}

/// Different proxies
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Proxy {
	/// The BungeeCord proxy
	BungeeCord,
	/// The Waterfall proxy
	Waterfall,
	/// The Velocity proxy
	Velocity,
	/// An unknown proxy
	#[cfg_attr(not(feature = "schema"), serde(untagged))]
	Unknown(String),
}

impl Proxy {
	/// Parse a Proxy from a string
	pub fn parse_from_str(string: &str) -> Self {
		match string {
			"bungeecord" => Self::BungeeCord,
			"waterfall" => Self::Waterfall,
			"velocity" => Self::Velocity,
			other => Self::Unknown(other.into()),
		}
	}
}

impl Display for Proxy {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::BungeeCord => write!(f, "BungeeCord"),
			Self::Waterfall => write!(f, "Waterfall"),
			Self::Velocity => write!(f, "Velocity"),
			Self::Unknown(other) => write!(f, "{other}"),
		}
	}
}

/// Matcher for different types of server proxies
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ProxyMatch {
	/// Matches any proxy that can load BungeeCord plugins
	BungeeCordLike,
	/// Matches a specific proxy
	#[serde(untagged)]
	Proxy(Proxy),
}

impl ProxyMatch {
	/// Parse a ProxyMatch from a string
	pub fn parse_from_str(string: &str) -> Self {
		match string {
			"bungeecordlike" => Self::BungeeCordLike,
			other => Self::Proxy(Proxy::parse_from_str(other)),
		}
	}

	/// Checks if a proxy matches
	pub fn matches(&self, other: &Proxy) -> bool {
		match self {
			Self::BungeeCordLike => matches!(other, Proxy::BungeeCord | Proxy::Waterfall),
			Self::Proxy(proxy) => proxy == other,
		}
	}
}
