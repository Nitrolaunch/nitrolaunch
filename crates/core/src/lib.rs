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

/// Different types of accounts and authentication
pub mod account;
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
use nitro_shared::UpdateDepth;
use tokio::sync::Mutex;
use util::versions::MinecraftVersion;
use version::{
	InstalledVersion, LoadVersionManifestParameters, LoadVersionParameters, VersionParameters,
	VersionRegistry,
};

pub use config::{ConfigBuilder, Configuration};
pub use instance::{ClientWindowConfig, Instance, InstanceConfiguration, InstanceKind};
pub use io::files::paths::Paths;
pub use launch::{InstanceHandle, QuickPlayType, WrapperCommand};

use crate::io::java::install::{CustomJavaFunction, JavaInstallationRegistry};

/// Wrapper around all usage of `nitro_core`
pub struct NitroCore {
	config: Configuration,
	paths: Arc<Paths>,
	req_client: reqwest::Client,
	persistent: Arc<Mutex<PersistentData>>,
	versions: VersionRegistry,
	java_installations: JavaInstallationRegistry,
	custom_java_fn: Option<Arc<dyn CustomJavaFunction>>,
}

impl NitroCore {
	/// Construct a new core with set configuration
	pub fn with_config(config: Configuration) -> anyhow::Result<Self> {
		Self::with_config_and_paths(config, Paths::new().context("Failed to create core paths")?)
	}

	/// Construct a new core with set configuration and paths
	pub fn with_config_and_paths(config: Configuration, paths: Paths) -> anyhow::Result<Self> {
		let persistent =
			PersistentData::open(&paths).context("Failed to open persistent data file")?;
		let out = Self {
			paths: Arc::new(paths),
			req_client: reqwest::Client::new(),
			persistent: Arc::new(Mutex::new(persistent)),
			versions: VersionRegistry::new(),
			config,
			java_installations: JavaInstallationRegistry {
				installations: Arc::new(Mutex::new(HashMap::new())),
			},
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

	/// Get the version manifest
	pub async fn get_version_manifest(
		&self,
		requested_version: Option<&MinecraftVersion>,
		depth: UpdateDepth,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<&Arc<VersionManifestAndList>> {
		let params = LoadVersionManifestParameters {
			requested_version,
			paths: &self.paths,
			update_manager: &UpdateManager::new(depth),
			req_client: &self.req_client,
		};
		self.versions.load_version_manifest(params, o).await
	}

	/// Load or install a version of the game
	pub async fn get_version(
		&self,
		version: &MinecraftVersion,
		depth: UpdateDepth,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<InstalledVersion> {
		let version_manifest = self
			.get_version_manifest(Some(version), depth, o)
			.await
			.context("Failed to ensure version manifest exists")?;
		let version = version
			.get_version(&version_manifest.manifest)
			.context("Latest release or snapshot is not present in manifest")?;

		let manager = UpdateManager::new(depth);

		let params = LoadVersionParameters {
			paths: &self.paths,
			req_client: &self.req_client,
			update_manager: &manager,
		};
		let inner = self
			.versions
			.get_version(&version, params, o)
			.await
			.context("Failed to get or install version")?;

		let params = VersionParameters {
			paths: self.paths.clone(),
			req_client: self.req_client.clone(),
			persistent: self.persistent.clone(),
			update_manager: manager.clone(),
			java_installations: self.java_installations.clone(),
			censor_secrets: self.config.censor_secrets,
			disable_hardlinks: self.config.disable_hardlinks,
			branding: self.config.branding.clone(),
			custom_java_fn: self.custom_java_fn.clone(),
		};
		Ok(InstalledVersion { inner, params })
	}

	/// Get just the VersionInfo for a version, without creating the version.
	/// This is useful for doing your own installation of things. This will download
	/// the version manifest if it is not downloaded already
	pub async fn get_version_info(
		&self,
		version: &MinecraftVersion,
		depth: UpdateDepth,
	) -> anyhow::Result<VersionInfo> {
		let mut o = output::NoOp;
		let manifest = self
			.get_version_manifest(Some(version), depth, &mut o)
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
	pub async fn get_java_installation(
		&self,
		major_version: JavaMajorVersion,
		kind: JavaInstallationKind,
		depth: UpdateDepth,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<JavaInstallation> {
		let key = (kind.clone(), major_version);

		if let Some(existing) = self.java_installations.installations.lock().await.get(&key) {
			return Ok(existing.clone());
		}

		let java_params = JavaInstallParameters {
			paths: &self.paths,
			update_manager: &UpdateManager::new(depth),
			persistent: self.persistent.clone(),
			req_client: &self.req_client,
			custom_install_func: self.custom_java_fn.clone(),
		};
		let java = JavaInstallation::install(kind.clone(), major_version, java_params, o)
			.await
			.context("Failed to install or update Java")?;

		self.java_installations
			.installations
			.lock()
			.await
			.insert(key.clone(), java.clone());

		Ok(java)
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
