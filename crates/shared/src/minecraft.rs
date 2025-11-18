use serde::{Deserialize, Serialize};

use crate::util::DefaultExt;
use crate::versions::VersionName;

/// JSON format for the version manifest that contains all available Minecraft versions
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionManifest {
	/// The latest available versions
	#[serde(default)]
	pub latest: Option<LatestVersions>,
	/// The list of available versions, from newest to oldest
	pub versions: Vec<VersionEntry>,
}

/// Entry for a version in the version manifest
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionEntry {
	/// The identifier for the version (e.g. "1.19.2" or "22w13a")
	pub id: String,
	/// What type of version this is
	#[serde(rename = "type")]
	#[serde(default)]
	pub ty: VersionType,
	/// The URL to the client version meta for this version
	pub url: String,
	/// Whether the client meta needs to be unzipped first
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub is_zipped: bool,
	/// The name of the source for this version, which can be used by plugins
	/// to show that the version is from that plugin
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub source: Option<String>,
}

/// Type of a version in the version manifest
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
	/// A release version
	#[default]
	Release,
	/// A snapshot / development version
	Snapshot,
	/// An old alpha version
	OldAlpha,
	/// An old beta version
	OldBeta,
	/// An unknown version type
	#[serde(untagged)]
	Other(String),
}

/// Latest available Minecraft versions in the version manifest
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LatestVersions {
	/// The latest release version
	pub release: VersionName,
	/// The latest snapshot version
	pub snapshot: VersionName,
}

/// Struct for a Minecraft Profile from the Minecraft Services API
#[derive(Deserialize, Serialize, Debug)]
pub struct MinecraftUserProfile {
	/// The username of this user
	pub name: String,
	/// The UUID of this user
	#[serde(rename = "id")]
	pub uuid: String,
	/// The list of skins that this user has
	pub skins: Vec<Skin>,
	/// The list of capes that this user has
	pub capes: Vec<Cape>,
}

/// A skin for a Minecraft user
#[derive(Deserialize, Serialize, Debug)]
pub struct Skin {
	/// Common cosmetic data for the skin
	#[serde(flatten)]
	pub cosmetic: Cosmetic,
	/// What variant of skin this is
	pub variant: SkinVariant,
}

/// Variant for a skin
#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SkinVariant {
	/// The classic wide-arm player model
	Classic,
	/// The newer slim player model
	Slim,
}

/// A cape for a Minecraft user
#[derive(Deserialize, Serialize, Debug)]
pub struct Cape {
	/// Common cosmetic data for the cape
	#[serde(flatten)]
	pub cosmetic: Cosmetic,
	/// The codename for this cape, such as 'migrator'
	pub alias: String,
}

/// Common structure used for a user cosmetic (skins and capes)
#[derive(Deserialize, Serialize, Debug)]
pub struct Cosmetic {
	/// The ID of this cosmetic
	pub id: String,
	/// The URL to the cosmetic image file
	pub url: String,
	/// The state of the cosmetic
	pub state: CosmeticState,
}

/// State for a cosmetic of whether it is active or not
#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CosmeticState {
	/// The cosmetic is active and being used
	Active,
	/// The cosmetic is not active
	Inactive,
}
