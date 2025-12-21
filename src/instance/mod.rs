/// Addon-related functions for instances
mod addons;
/// Launching an instance
pub mod launch;
/// Storing install data about an instance like the current version and packages
pub mod lock;
/// Managing and installing packages on an instance
pub mod packages;
/// Setup of instance contents
pub mod setup;
/// Keeping track of running instance processes
pub mod tracking;
/// Import and export of instances to other formats
pub mod transfer;
/// Updating an instance
pub mod update;
/// Updating shared world files
pub mod world_files;

use std::path::PathBuf;

use nitro_config::instance::{ClientWindowConfig, InstanceConfig};
use nitro_core::util::versions::MinecraftVersion;
use nitro_pkg::overrides::PackageOverrides;
use nitro_shared::later::Later;
use nitro_shared::loaders::Loader;
use nitro_shared::pkg::PackageStability;
use nitro_shared::versions::VersionPattern;
use nitro_shared::Side;

use crate::io::paths::Paths;

use self::launch::LaunchOptions;
use self::setup::{InstanceDirs, ModificationData};

use super::config::package::PackageConfig;
use nitro_shared::id::InstanceID;

/// An instance of the game on a template
#[derive(Debug)]
pub struct Instance {
	/// What type of instance this is
	pub(crate) kind: InstKind,
	/// The ID of this instance
	pub(crate) id: InstanceID,
	/// Directories of the instance
	pub(crate) dirs: Later<InstanceDirs>,
	/// Configuration for the instance
	pub(crate) config: InstanceStoredConfig,
	/// Modification data
	modification_data: ModificationData,
}

/// Different kinds of instances and their associated data
#[derive(Debug, Clone)]
pub enum InstKind {
	/// A client instance
	Client {
		/// Configuration for the client window
		window: ClientWindowConfig,
	},
	/// A server instance
	Server {
		/// The new world name if it is changed by the options
		world_name: Option<String>,
	},
}

impl InstKind {
	/// Create a new client InstKind
	pub fn client(window: ClientWindowConfig) -> Self {
		Self::Client { window }
	}

	/// Create a new server InstKind
	pub fn server() -> Self {
		Self::Server { world_name: None }
	}

	/// Convert to the Side enum
	pub fn to_side(&self) -> Side {
		match self {
			Self::Client { .. } => Side::Client,
			Self::Server { .. } => Side::Server,
		}
	}
}

/// The stored configuration on an instance
#[derive(Debug)]
pub struct InstanceStoredConfig {
	/// The instance display name
	pub name: Option<String>,
	/// A path to an icon for the instance
	pub icon: Option<String>,
	/// The Minecraft version
	pub version: MinecraftVersion,
	/// Loader for the instance
	pub loader: Loader,
	/// Version for the loader
	pub loader_version: Option<VersionPattern>,
	/// Launch options for the instance
	pub launch: LaunchOptions,
	/// The instance's global datapack folder
	pub datapack_folder: Option<String>,
	/// The packages on the instance, consolidated from all parent sources
	pub packages: Vec<PackageConfig>,
	/// Default stability for packages
	pub package_stability: PackageStability,
	/// Package overrides
	pub package_overrides: PackageOverrides,
	/// Game dir override
	pub game_dir: Option<PathBuf>,
	/// Whether custom launch behavior is enabled
	pub custom_launch: bool,
	/// The original instance configuration before applying templates
	pub original_config: InstanceConfig,
	/// The original instance configuration after applying templates
	pub original_config_with_templates: InstanceConfig,
	/// The original instance configuration after applying templates and plugins
	pub original_config_with_templates_and_plugins: InstanceConfig,
	/// Custom plugin config
	pub plugin_config: serde_json::Map<String, serde_json::Value>,
}

impl Instance {
	/// Create a new instance
	pub fn new(kind: InstKind, id: InstanceID, config: InstanceStoredConfig) -> Self {
		Self {
			kind,
			id,
			config,
			dirs: Later::Empty,
			modification_data: ModificationData::new(),
		}
	}

	/// Get the kind of the instance
	pub fn get_kind(&self) -> &InstKind {
		&self.kind
	}

	/// Get the side of the instance
	pub fn get_side(&self) -> Side {
		self.kind.to_side()
	}

	/// Get the ID of the instance
	pub fn get_id(&self) -> &InstanceID {
		&self.id
	}

	/// Get the instance's directories
	pub fn get_dirs(&self) -> &Later<InstanceDirs> {
		&self.dirs
	}

	/// Get the instance's stored configuration
	pub fn get_config(&self) -> &InstanceStoredConfig {
		&self.config
	}
}

/// Deletes files for the given instance ID, including saves. Use with caution!!!
pub async fn delete_instance_files(instance_id: &str, paths: &Paths) -> anyhow::Result<()> {
	let path = paths.data.join("instances").join(instance_id);
	if path.exists() {
		tokio::fs::remove_dir_all(path).await?;
	}

	Ok(())
}
