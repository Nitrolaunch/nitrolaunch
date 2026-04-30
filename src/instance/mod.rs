/// Addon-related functions for instances
mod addons;
/// Launching an instance
pub mod launch;
/// Accessing log files
pub mod logs;
/// Operations on the instance, like deleting, modifying, or querying files
pub mod operations;
/// Managing and installing packages on an instance
pub mod packages;
/// Keeping track of running instance processes
pub mod tracking;
/// Import and export of instances to other formats
pub mod transfer;
/// Updating an instance
pub mod update;
/// Updating shared world files
pub mod world_files;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use nitro_config::instance::{
	ClientWindowConfig, InstanceConfig, LaunchConfig, LaunchMemory, is_valid_instance_id,
};
use nitro_config::template::TemplateConfig;
use nitro_core::io::java::install::JavaInstallationKind;
use nitro_core::util::versions::MinecraftVersion;
use nitro_instance::get_instance_dir;
use nitro_instance::lock::InstanceLockfile;
use nitro_shared::Side;
use nitro_shared::java_args::MemoryNum;
use nitro_shared::loaders::Loader;
use nitro_shared::versions::{VersionPattern, parse_versioned_string};

use crate::config::package::read_package_config;
use crate::io::paths::Paths;

use self::launch::LaunchOptions;
use self::update::setup::ModificationData;

use super::config::package::PackageConfig;
use nitro_shared::id::{InstanceID, TemplateID};

/// An instance of the game on a template
#[derive(Debug)]
pub struct Instance {
	/// What type of instance this is
	kind: InstKind,
	/// The ID of this instance
	id: InstanceID,
	/// Directory for the instance's files
	dir: Option<PathBuf>,
	/// The Minecraft version
	version: MinecraftVersion,
	/// Loader for the instance
	loader: Loader,
	/// Version for the loader
	loader_version: VersionPattern,
	/// Launch options for the instance
	launch: LaunchOptions,
	/// The packages on the instance, consolidated from all parent sources
	packages: Vec<PackageConfig>,
	/// The instance configuration after applying templates
	config: InstanceConfig,
	/// The original instance configuration before applying templates
	original_config: InstanceConfig,
	/// Modification data
	modification_data: ModificationData,
}

impl Instance {
	/// Create a new instance from configuration
	pub fn from_config(
		id: InstanceID,
		mut config: InstanceConfig,
		templates: &HashMap<TemplateID, TemplateConfig>,
		paths: &Paths,
	) -> anyhow::Result<Self> {
		if !is_valid_instance_id(&id) {
			bail!("Invalid instance ID '{}'", id);
		}

		let original_config = config.clone();
		let config = config.apply_templates(templates)?;

		let kind = match config.side.unwrap() {
			Side::Client => InstKind::client(config.window.clone()),
			Side::Server => InstKind::server(),
		};

		let (loader, loader_version) = if let Some(loader) = &config.loader {
			parse_loader_config(loader)
		} else {
			(Loader::Vanilla, VersionPattern::Any)
		};

		let version = MinecraftVersion::from_deser(
			&config
				.version
				.clone()
				.context("Instance is missing a Minecraft version")?,
		);

		let read_packages = config
			.packages
			.clone()
			.into_iter()
			.map(|x| read_package_config(x, config.package_stability.unwrap_or_default()))
			.collect();

		let base_dir = paths.data.join("instances").join(&*id);

		let inst_dir = if let Some(inst_dir) = &config.dir {
			// 'none' can be used to specify a missing game dir
			if inst_dir == "none" {
				None
			} else {
				Some(inst_dir.into())
			}
		} else {
			Some(get_instance_dir(&base_dir, kind.to_side()))
		};

		Ok(Self {
			dir: inst_dir,
			kind,
			id,
			launch: launch_config_to_options(config.launch.clone())?,
			version,
			loader,
			loader_version,
			packages: read_packages,
			original_config,
			config,
			modification_data: ModificationData::new(),
		})
	}

	/// Get the kind of the instance
	pub fn kind(&self) -> &InstKind {
		&self.kind
	}

	/// Get the side of the instance
	pub fn side(&self) -> Side {
		self.kind.to_side()
	}

	/// Get the ID of the instance
	pub fn id(&self) -> &InstanceID {
		&self.id
	}

	/// Get the instance's directory
	pub fn dir(&self) -> Option<&Path> {
		self.dir.as_deref()
	}

	/// Get the instance's version
	pub fn version(&self) -> &MinecraftVersion {
		&self.version
	}

	/// Get the instance's loader
	pub fn loader(&self) -> &Loader {
		&self.loader
	}

	/// Get the instance's loader version
	pub fn loader_version(&self) -> &VersionPattern {
		&self.loader_version
	}

	/// Get the instance's stored configuration
	pub fn config(&self) -> &InstanceConfig {
		&self.config
	}

	/// Get the original, editable config before templates are applied
	pub fn original_config(&self) -> &InstanceConfig {
		&self.config
	}

	/// Opens the lockfile for this instance and returns it
	pub fn get_lockfile(&self, paths: &Paths) -> anyhow::Result<InstanceLockfile> {
		let lock_path = InstanceLockfile::get_path(self.dir.as_deref(), &self.id, &paths.internal);
		InstanceLockfile::open(&lock_path)
	}

	/// Checks whether the lockfile for this instance exists, letting you check whether it has been successfully updated
	pub fn lockfile_exists(&self, paths: &Paths) -> bool {
		let lock_path = InstanceLockfile::get_path(self.dir.as_deref(), &self.id, &paths.internal);
		lock_path.exists()
	}
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

/// Parses a loader from configuration
pub fn parse_loader_config(loader: &str) -> (Loader, VersionPattern) {
	let (loader, version) = parse_versioned_string(loader);
	(Loader::parse_from_str(loader), version)
}

fn launch_config_to_options(config: LaunchConfig) -> anyhow::Result<LaunchOptions> {
	let min_mem = match &config.memory {
		LaunchMemory::None => None,
		LaunchMemory::Single(string) => MemoryNum::parse(string),
		LaunchMemory::Both { min, .. } => MemoryNum::parse(min),
	};
	let max_mem = match &config.memory {
		LaunchMemory::None => None,
		LaunchMemory::Single(string) => MemoryNum::parse(string),
		LaunchMemory::Both { max, .. } => MemoryNum::parse(max),
	};
	if let Some(min_mem) = &min_mem {
		if let Some(max_mem) = &max_mem {
			ensure!(
				min_mem.to_bytes() <= max_mem.to_bytes(),
				"Minimum memory must be less than or equal to maximum memory"
			);
		}
	}
	Ok(LaunchOptions {
		jvm_args: config.args.jvm.parse(),
		game_args: config.args.game.parse(),
		min_mem,
		max_mem,
		java: JavaInstallationKind::parse(config.java.as_deref().unwrap_or("auto")),
		env: config.env,
		wrapper: config.wrapper,
		quick_play: config.quick_play,
		use_log4j_config: config.use_log4j_config,
	})
}
