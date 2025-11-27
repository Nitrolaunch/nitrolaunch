#![warn(missing_docs)]

//! This library is used by Nitrolaunch to install and launch Minecraft. It aims to be the most powerful, fast,
//! and correct implementation available, without being bloated with extra features. Implementations
//! for installing certain modifications, like modloaders and alternative server runtimes, will be
//! provided in extension plugins
//!
//! Note: The functions in this library expect the use of the Tokio runtime and may panic
//! if it is not used

use std::collections::HashMap;
use std::sync::Arc;

pub use nitro_auth as auth_crate;

/// Configuration for library functionality
pub mod config;
/// Instances of versions that can be launched
pub mod instance;
/// Input / output with data formats and the system
pub mod io;
/// Code for launching the game
pub mod launch;
/// Networking interfaces
pub mod net;
/// Different types of users and authentication
pub mod user;
/// Common utilities
pub mod util;
/// Installable versions of the game
pub mod version;

use anyhow::Context;
use io::java::install::{JavaInstallParameters, JavaInstallation, JavaInstallationKind};
use io::java::JavaMajorVersion;
use io::{persistent::PersistentData, update::UpdateManager};
use net::game_files::version_manifest::{make_version_list, VersionManifestAndList};
use nitro_shared::minecraft::VersionEntry;
use nitro_shared::output::{self, NitroOutput};
use nitro_shared::versions::VersionInfo;
use user::UserManager;
use util::versions::MinecraftVersion;
use version::{
	InstalledVersion, LoadVersionManifestParameters, LoadVersionParameters, VersionParameters,
	VersionRegistry,
};

pub use config::{ConfigBuilder, Configuration};
pub use instance::{ClientWindowConfig, Instance, InstanceConfiguration, InstanceKind};
pub use io::files::paths::Paths;
pub use launch::{InstanceHandle, QuickPlayType, WrapperCommand};

use crate::io::java::install::CustomJavaFunction;

/// Wrapper around all usage of `nitro_core`
pub struct NitroCore {
	config: Configuration,
	paths: Paths,
	req_client: reqwest::Client,
	persistent: PersistentData,
	update_manager: UpdateManager,
	versions: VersionRegistry,
	users: UserManager,
	java_installations: HashMap<(JavaInstallationKind, JavaMajorVersion), JavaInstallation>,
	custom_java_fn: Option<Arc<dyn CustomJavaFunction>>,
}

impl NitroCore {
	/// Construct a new core with default settings
	pub fn new() -> anyhow::Result<Self> {
		Self::with_config(Configuration::new())
	}

	/// Construct a new core with set configuration
	pub fn with_config(config: Configuration) -> anyhow::Result<Self> {
		Self::with_config_and_paths(config, Paths::new().context("Failed to create core paths")?)
	}

	/// Construct a new core with set configuration and paths
	pub fn with_config_and_paths(config: Configuration, paths: Paths) -> anyhow::Result<Self> {
		let persistent =
			PersistentData::open(&paths).context("Failed to open persistent data file")?;
		let out = Self {
			paths,
			req_client: reqwest::Client::new(),
			persistent,
			update_manager: UpdateManager::new(config.update_depth),
			versions: VersionRegistry::new(),
			users: UserManager::new(config.ms_client_id.clone()),
			config,
			java_installations: HashMap::new(),
			custom_java_fn: None,
		};
		Ok(out)
	}

	/// Get the configuration that the core uses
	pub fn get_config(&self) -> &Configuration {
		&self.config
	}

	/// Set the reqwest client to be used if you already have one
	pub fn set_client(&mut self, req_client: reqwest::Client) {
		self.req_client = req_client;
	}

	/// Get the reqwest client that the core uses
	#[inline]
	pub fn get_client(&self) -> &reqwest::Client {
		&self.req_client
	}

	/// Get the paths that the core uses
	#[inline]
	pub fn get_paths(&self) -> &Paths {
		&self.paths
	}

	/// Get the UserManager in order to add, remove, and auth users
	pub fn get_users(&mut self) -> &mut UserManager {
		&mut self.users
	}

	/// Get the UpdateManager in order to help with custom installation
	/// routines
	pub fn get_update_manager(&self) -> &UpdateManager {
		&self.update_manager
	}

	/// Get the UpdateManager mutably in order to help with custom installation
	/// routines. Don't modify this unless you know what you are doing!
	pub fn get_update_manager_mut(&mut self) -> &mut UpdateManager {
		&mut self.update_manager
	}

	/// Get the version manifest
	pub async fn get_version_manifest(
		&mut self,
		requested_version: Option<&MinecraftVersion>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<&Arc<VersionManifestAndList>> {
		let params = LoadVersionManifestParameters {
			requested_version,
			paths: &self.paths,
			update_manager: &self.update_manager,
			req_client: &self.req_client,
		};
		self.versions.load_version_manifest(params, o).await
	}

	/// Load or install a version of the game
	pub async fn get_version(
		&'_ mut self,
		version: &MinecraftVersion,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<InstalledVersion<'_, '_>> {
		let version_manifest = self
			.get_version_manifest(Some(version), o)
			.await
			.context("Failed to ensure version manifest exists")?;
		let version = version
			.get_version(&version_manifest.manifest)
			.context("Latest release or snapshot is not present in manifest")?;

		let params = LoadVersionParameters {
			paths: &self.paths,
			req_client: &self.req_client,
			update_manager: &self.update_manager,
		};
		let inner = self
			.versions
			.get_version(&version, params, o)
			.await
			.context("Failed to get or install version")?;

		let params = VersionParameters {
			paths: &self.paths,
			req_client: &self.req_client,
			persistent: &mut self.persistent,
			update_manager: &mut self.update_manager,
			users: &mut self.users,
			java_installations: &mut self.java_installations,
			censor_secrets: self.config.censor_secrets,
			disable_hardlinks: self.config.disable_hardlinks,
			branding: &self.config.branding,
			custom_java_fn: self.custom_java_fn.as_ref(),
		};
		Ok(InstalledVersion { inner, params })
	}

	/// Get just the VersionInfo for a version, without creating the version.
	/// This is useful for doing your own installation of things. This will download
	/// the version manifest if it is not downloaded already
	pub async fn get_version_info(
		&mut self,
		version: &MinecraftVersion,
	) -> anyhow::Result<VersionInfo> {
		let mut o = output::NoOp;
		let manifest = self
			.get_version_manifest(Some(version), &mut o)
			.await
			.context("Failed to get version manifest")?;
		let version = version
			.get_version(&manifest.manifest)
			.context("Version does not exist")?;
		let list = make_version_list(&manifest.manifest);
		Ok(VersionInfo {
			version: version.to_string(),
			versions: list,
		})
	}

	/// Installs Java
	pub async fn get_java_installation<'this>(
		&'this mut self,
		major_version: JavaMajorVersion,
		kind: JavaInstallationKind,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<&'this JavaInstallation> {
		let key = (kind.clone(), major_version);

		if self.java_installations.contains_key(&key) {
			return Ok(self.java_installations.get(&key).unwrap());
		}

		let java_params = JavaInstallParameters {
			paths: &self.paths,
			update_manager: &mut self.update_manager,
			persistent: &mut self.persistent,
			req_client: &self.req_client,
			custom_install_func: self.custom_java_fn.as_ref(),
		};
		let java = JavaInstallation::install(kind.clone(), major_version, java_params, o)
			.await
			.context("Failed to install or update Java")?;

		self.java_installations.insert(key.clone(), java);

		Ok(self.java_installations.get(&key).unwrap())
	}

	/// Add additional versions to the version manifest. Must be called before the version manifest is obtained,
	/// including before creating any versions
	pub fn add_additional_versions(&mut self, versions: Vec<VersionEntry>) {
		self.versions.add_additional_versions(versions);
	}

	/// Set a custom Java installation function for unknown installations
	pub fn set_custom_java_install_fn(&mut self, func: Arc<dyn CustomJavaFunction>) {
		self.custom_java_fn = Some(func);
	}
}
