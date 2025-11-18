use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use nitro_shared::later::Later;
use nitro_shared::minecraft::{VersionEntry, VersionManifest};
use nitro_shared::output::NitroOutput;
use nitro_shared::output::{MessageContents, MessageLevel};
use nitro_shared::versions::{VersionInfo, VersionName};
use nitro_shared::Side;

use crate::config::BrandingProperties;
use crate::instance::{Instance, InstanceConfiguration, InstanceParameters};
use crate::io::files::paths::Paths;
use crate::io::java::install::{JavaInstallParameters, JavaInstallation, JavaInstallationKind};
use crate::io::java::JavaMajorVersion;
use crate::io::persistent::PersistentData;
use crate::io::update::UpdateManager;
use crate::net::game_files::client_meta::{self, ClientMeta};
use crate::net::game_files::version_manifest::{self, VersionManifestAndList};
use crate::net::game_files::{assets, game_jar, libraries};
use crate::user::UserManager;
use crate::util::versions::MinecraftVersion;

/// An installed version of the game. This cannot be constructed directly,
/// only from the NitroCore struct by using the `get_version()` method
pub struct InstalledVersion<'inner, 'params> {
	pub(crate) inner: &'inner mut InstalledVersionInner,
	pub(crate) params: VersionParameters<'params>,
}

impl InstalledVersion<'_, '_> {
	/// Get the version name
	pub fn get_version(&self) -> &VersionName {
		&self.inner.version
	}

	/// Get the client meta
	pub fn get_client_meta(&self) -> &ClientMeta {
		&self.inner.client_meta
	}

	/// Get the version info
	#[must_use]
	pub fn get_version_info(&self) -> VersionInfo {
		VersionInfo {
			version: self.inner.version.to_string(),
			versions: self.inner.version_manifest.list.clone(),
		}
	}

	/// Create an instance and its files using this version,
	/// ready to be launched
	pub async fn get_instance(
		&mut self,
		config: InstanceConfiguration,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Instance> {
		let params = InstanceParameters {
			version: &self.inner.version,
			version_manifest: &self.inner.version_manifest,
			paths: self.params.paths,
			req_client: self.params.req_client,
			persistent: self.params.persistent,
			update_manager: self.params.update_manager,
			client_meta: &self.inner.client_meta,
			users: self.params.users,
			java_installations: self.params.java_installations,
			client_assets_and_libs: &mut self.inner.client_assets_and_libs,
			censor_secrets: self.params.censor_secrets,
			disable_hardlinks: self.params.disable_hardlinks,
			branding: self.params.branding,
		};
		let instance = Instance::load(config, params, o)
			.await
			.context("Failed to load instance")?;
		Ok(instance)
	}

	/// Ensure that assets and libraries for the client are
	/// installed for this version. You shouldn't need to call this
	/// as these files will be automatically installed when creating a client
	/// instance
	pub async fn ensure_client_assets_and_libs(
		&mut self,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let params = ClientAssetsAndLibsParameters {
			client_meta: &self.inner.client_meta,
			version: &self.inner.version,
			paths: self.params.paths,
			req_client: self.params.req_client,
			version_manifest: &self.inner.version_manifest,
			update_manager: self.params.update_manager,
		};
		self.inner.client_assets_and_libs.load(params, o).await
	}

	/// Gets or installs a Java installation following the parameters of this version and the given installation
	pub async fn get_java_installation(
		&mut self,
		kind: JavaInstallationKind,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<JavaInstallation> {
		let key = (kind.clone(), self.inner.client_meta.java_info.major_version);

		if !self.params.java_installations.contains_key(&key) {
			let java_params = JavaInstallParameters {
				paths: self.params.paths,
				update_manager: self.params.update_manager,
				persistent: self.params.persistent,
				req_client: self.params.req_client,
			};

			let java = JavaInstallation::install(
				kind,
				self.inner.client_meta.java_info.major_version,
				java_params,
				o,
			)
			.await
			.context("Failed to install or update Java")?;

			self.params
				.java_installations
				.insert(key.clone(), java.clone());
		}

		Ok(self.params.java_installations.get(&key).unwrap().clone())
	}

	/// Gets the vanilla game JAR for the given side, returning the path to it
	pub async fn get_game_jar(
		&self,
		side: Side,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<PathBuf> {
		game_jar::get(
			side,
			&self.inner.client_meta,
			&self.inner.version,
			self.params.paths,
			self.params.update_manager,
			self.params.req_client,
			o,
		)
		.await
		.context("Failed to get the game JAR file")?;

		Ok(crate::io::minecraft::game_jar::get_path(
			side,
			&self.inner.version,
			None,
			self.params.paths,
		))
	}
}

pub(crate) struct InstalledVersionInner {
	version: VersionName,
	version_manifest: Arc<VersionManifestAndList>,
	client_meta: ClientMeta,
	client_assets_and_libs: ClientAssetsAndLibraries,
}

impl InstalledVersionInner {
	/// Load a version
	async fn load(
		version: VersionName,
		version_manifest: &Arc<VersionManifestAndList>,
		params: LoadVersionParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Self> {
		// Get the client meta
		o.start_process();
		o.display(
			MessageContents::StartProcess("Obtaining client metadata".into()),
			MessageLevel::Important,
		);

		let client_meta = client_meta::get(
			&version,
			&version_manifest.manifest,
			params.paths,
			params.update_manager,
			params.req_client,
			o,
		)
		.await
		.context("Failed to get client meta")?;

		o.display(
			MessageContents::Success("Client meta obtained".into()),
			MessageLevel::Important,
		);
		o.end_process();

		Ok(Self {
			version,
			version_manifest: version_manifest.clone(),
			client_meta,
			client_assets_and_libs: ClientAssetsAndLibraries::new(),
		})
	}
}

/// A registry of installed versions
pub(crate) struct VersionRegistry {
	versions: HashMap<VersionName, InstalledVersionInner>,
	version_manifest: Later<Arc<VersionManifestAndList>>,
	additional_versions: Vec<VersionEntry>,
}

impl VersionRegistry {
	pub fn new() -> Self {
		Self {
			versions: HashMap::new(),
			version_manifest: Later::Empty,
			additional_versions: Vec::new(),
		}
	}

	/// Load a version if it is not already loaded, and get it otherwise
	pub async fn get_version(
		&mut self,
		version: &VersionName,
		params: LoadVersionParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<&mut InstalledVersionInner> {
		// Ensure the version manifest first
		let requested_version = MinecraftVersion::Version(version.clone());
		let vm_params = LoadVersionManifestParameters {
			requested_version: Some(&requested_version),
			paths: params.paths,
			req_client: params.req_client,
			update_manager: params.update_manager,
		};
		self.load_version_manifest(vm_params, o)
			.await
			.context("Failed to get version manifest")?;

		if !self.versions.contains_key(version) {
			let installed_version = InstalledVersionInner::load(
				version.clone(),
				self.version_manifest.get(),
				params,
				o,
			)
			.await?;
			self.versions.insert(version.clone(), installed_version);
		}
		Ok(self
			.versions
			.get_mut(version)
			.expect("Version should exist in map"))
	}

	/// Load the version manifest
	pub async fn load_version_manifest(
		&mut self,
		params: LoadVersionManifestParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<&Arc<VersionManifestAndList>> {
		if self.version_manifest.is_empty() {
			let mut manifest = version_manifest::get_with_output(
				params.requested_version,
				params.paths,
				params.update_manager,
				params.req_client,
				o,
			)
			.await
			.context("Failed to get version manifest")?;

			// Add additional versions
			let additional_versions = std::mem::take(&mut self.additional_versions);
			add_versions(&mut manifest, additional_versions);

			let combo = VersionManifestAndList::new(manifest);

			self.version_manifest.fill(Arc::new(combo));
		}
		Ok(self.version_manifest.get())
	}

	/// Add additional versions to the manifest. Must be called before the manifest is obtained.
	pub fn add_additional_versions(&mut self, versions: Vec<VersionEntry>) {
		self.additional_versions.extend(versions);
	}
}

/// Container struct for parameters for versions and instances
pub(crate) struct VersionParameters<'a> {
	pub paths: &'a Paths,
	pub req_client: &'a reqwest::Client,
	pub persistent: &'a mut PersistentData,
	pub update_manager: &'a mut UpdateManager,
	pub users: &'a mut UserManager,
	pub java_installations:
		&'a mut HashMap<(JavaInstallationKind, JavaMajorVersion), JavaInstallation>,
	pub censor_secrets: bool,
	pub disable_hardlinks: bool,
	pub branding: &'a BrandingProperties,
}

/// Container struct for parameters for loading version innards
#[derive(Clone)]
pub(crate) struct LoadVersionParameters<'a> {
	pub paths: &'a Paths,
	pub req_client: &'a reqwest::Client,
	pub update_manager: &'a UpdateManager,
}

/// Container struct for parameters for loading the version manifest
#[derive(Clone)]
pub(crate) struct LoadVersionManifestParameters<'a> {
	pub requested_version: Option<&'a MinecraftVersion>,
	pub paths: &'a Paths,
	pub req_client: &'a reqwest::Client,
	pub update_manager: &'a UpdateManager,
}

/// Data for client assets and libraries that are only
/// loaded when a client needs them
pub(crate) struct ClientAssetsAndLibraries {
	loaded: bool,
}

impl ClientAssetsAndLibraries {
	pub fn new() -> Self {
		Self { loaded: false }
	}

	pub async fn load(
		&mut self,
		params: ClientAssetsAndLibsParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		if self.loaded {
			return Ok(());
		}
		let result = assets::get(
			params.client_meta,
			params.paths,
			params.version,
			&params.version_manifest.list,
			params.update_manager,
			params.req_client,
			o,
		)
		.await
		.context("Failed to get game assets")?;
		params.update_manager.add_result(result);

		let result = libraries::get(
			&params.client_meta.libraries,
			&params.paths.internal,
			params.version,
			params.update_manager,
			params.req_client,
			o,
		)
		.await
		.context("Failed to get game libraries")?;
		params.update_manager.add_result(result);

		self.loaded = true;
		Ok(())
	}
}

/// Container struct for parameters for loading client assets and libraries
pub(crate) struct ClientAssetsAndLibsParameters<'a> {
	pub client_meta: &'a ClientMeta,
	pub version: &'a VersionName,
	pub paths: &'a Paths,
	pub req_client: &'a reqwest::Client,
	pub version_manifest: &'a VersionManifestAndList,
	pub update_manager: &'a mut UpdateManager,
}

/// Adds extra versions to a manifest
pub fn add_versions(manifest: &mut VersionManifest, additional_versions: Vec<VersionEntry>) {
	// Versions with the same name should replace existing ones in the manifest
	for new_version in additional_versions {
		if let Some(pos) = manifest
			.versions
			.iter()
			.position(|x| x.id == new_version.id)
		{
			manifest.versions[pos] = new_version;
		} else {
			manifest.versions.insert(0, new_version);
		}
	}
}
