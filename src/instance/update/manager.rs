use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use nitro_core::auth_crate::mc::ClientId;
use nitro_core::config::BrandingProperties;
use nitro_core::net::game_files::version_manifest::VersionManifestAndList;
use nitro_core::net::minecraft::MinecraftUserProfile;
use nitro_core::user::{CustomAuthFunction, UserManager};
use nitro_core::util::versions::MinecraftVersion;
use nitro_core::version::InstalledVersion;
use nitro_core::NitroCore;
use nitro_plugin::hooks::{AddVersions, HandleAuth, HandleAuthArg};
use nitro_shared::later::Later;
use nitro_shared::output::NitroOutput;
use nitro_shared::output::NoOp;
use nitro_shared::versions::VersionInfo;
use nitro_shared::UpdateDepth;
use reqwest::Client;

use crate::io::paths::Paths;
use crate::plugin::PluginManager;

/// Requirements for operations that may be shared by multiple instances in a profile
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum UpdateRequirement {
	/// Client logging configuration
	ClientLoggingConfig,
}

/// Settings for updating
#[derive(Debug)]
pub struct UpdateSettings {
	/// The depth to perform updates at
	pub depth: UpdateDepth,
	/// Whether to do offline authentication
	pub offline_auth: bool,
}

/// Manager for when we are updating profile files.
/// It will keep track of files we have already downloaded, manage task requirements, etc
pub struct UpdateManager {
	/// Settings for the update
	pub settings: UpdateSettings,
	/// Update requirements that are fulfilled
	requirements: HashSet<UpdateRequirement>,
	/// File paths that are added when they have been updated by other functions
	files: HashSet<PathBuf>,
	/// The Minecraft version of the manager
	mc_version: Later<MinecraftVersion>,
	/// The MS client id, if used
	ms_client_id: Option<ClientId>,
	/// The core to be fulfilled later
	pub core: Later<NitroCore>,
	/// The version info to be fulfilled later
	pub version_info: Later<VersionInfo>,
	/// The version manifets to be fulfilled later
	pub version_manifest: Later<Arc<VersionManifestAndList>>,
}

impl UpdateManager {
	/// Create a new UpdateManager
	pub fn new(depth: UpdateDepth) -> Self {
		let settings = UpdateSettings {
			depth,
			offline_auth: false,
		};

		Self {
			settings,
			requirements: HashSet::new(),
			core: Later::Empty,
			ms_client_id: None,
			files: HashSet::new(),
			version_info: Later::Empty,
			mc_version: Later::Empty,
			version_manifest: Later::Empty,
		}
	}

	/// Set offline authentication
	pub fn offline_auth(&mut self) {
		self.settings.offline_auth = true;
	}

	/// Set the MS client ID
	pub fn set_client_id(&mut self, id: ClientId) {
		self.ms_client_id = Some(id);
	}

	/// Add a single requirement
	pub fn add_requirement(&mut self, req: UpdateRequirement) {
		self.requirements.insert(req);
	}

	/// Add multiple requirements
	pub fn add_requirements(&mut self, reqs: HashSet<UpdateRequirement>) {
		self.requirements.extend(reqs);
	}

	/// Check if a requirement is held
	pub fn has_requirement(&self, req: UpdateRequirement) -> bool {
		self.requirements.contains(&req)
	}

	/// Add tracked files to the manager
	pub fn add_files(&mut self, files: HashSet<PathBuf>) {
		self.files.extend(files);
	}

	/// Adds an UpdateMethodResult to the UpdateManager
	pub fn add_result(&mut self, result: UpdateMethodResult) {
		self.add_files(result.files_updated);
	}

	/// Whether a file needs to be updated
	pub fn should_update_file(&self, file: &Path) -> bool {
		if self.settings.depth == UpdateDepth::Force {
			!self.files.contains(file) || !file.exists()
		} else {
			!file.exists()
		}
	}

	/// Set the Minecraft version. Can be used with the same UpdateManager and will work fine.
	/// Just make sure to fulfill requirements again.
	pub fn set_version(&mut self, version: &MinecraftVersion) {
		self.mc_version.fill(version.clone());
		// We have to clear these now since they are out of date
		self.version_info.clear();
	}

	/// Run all of the operations that are part of the requirements.
	pub async fn fulfill_requirements(
		&mut self,
		users: &UserManager,
		plugins: &PluginManager,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		// Setup the core
		self.setup_core(client, users, plugins, paths, o)
			.await
			.context("Failed to setup core")?;

		// If the Minecraft version is not set then we can just assume it is not being used
		if self.mc_version.is_empty() {
			return Ok(());
		}

		let version = self
			.get_core_version(o)
			.await
			.context("Failed to get version")?;

		let version_info = version.get_version_info();

		self.version_info.fill(version_info);

		Ok(())
	}

	/// Sets up the core
	async fn setup_core(
		&mut self,
		client: &Client,
		users: &UserManager,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		if self.core.is_full() {
			return Ok(());
		}

		// Setup the core
		let mut core_config = nitro_core::ConfigBuilder::new()
			.update_depth(self.settings.depth)
			.branding(BrandingProperties::new(
				"Nitrolaunch".into(),
				crate::VERSION.into(),
			));
		if let Some(client_id) = &self.ms_client_id {
			core_config = core_config.ms_client_id(client_id.clone());
		}
		let core_config = core_config.build();
		let mut core = NitroCore::with_config(core_config).context("Failed to initialize core")?;

		// Set up user manager along with custom auth function that handles using plugins
		core.get_users().steal_users(users);
		core.get_users().set_offline(self.settings.offline_auth);
		{
			let plugins = plugins.clone();
			let paths = paths.clone();

			core.get_users()
				.set_custom_auth_function(Arc::new(AuthFunction { plugins, paths }));
		}

		core.set_client(client.clone());

		// Add extra versions to manifest from plugins
		let results = plugins
			.call_hook(AddVersions, &(), paths, o)
			.await
			.context("Failed to call add_versions hook")?;
		for result in results {
			let result = result.result(o).await?;
			core.add_additional_versions(result);
		}

		let version_manifest = core.get_version_manifest(None, &mut NoOp).await?;
		self.version_manifest.fill(version_manifest.clone());

		self.core.fill(core);

		Ok(())
	}

	/// Get the version from the core
	pub async fn get_core_version(
		&mut self,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<InstalledVersion> {
		let version = self
			.core
			.get_mut()
			.get_version(self.mc_version.get_mut(), o)
			.await
			.context("Failed to get core version")?;

		Ok(version)
	}
}

/// CustomAuthFunction implementation for user types using plugins
struct AuthFunction {
	plugins: PluginManager,
	paths: Paths,
}

#[async_trait::async_trait]
impl CustomAuthFunction for AuthFunction {
	async fn auth(
		&self,
		id: &str,
		user_type: &str,
	) -> anyhow::Result<Option<MinecraftUserProfile>> {
		let arg = HandleAuthArg {
			user_id: id.to_string(),
			user_type: user_type.to_string(),
		};
		let results = self
			.plugins
			.call_hook(HandleAuth, &arg, &self.paths, &mut NoOp)
			.await
			.context("Failed to call handle auth hook")?;
		for result in results {
			let result = result.result(&mut NoOp).await?;
			if result.handled {
				return Ok(result.profile);
			}
		}

		Ok(None)
	}
}

/// Struct returned by updating functions, with data like changed files
#[derive(Default)]
pub struct UpdateMethodResult {
	/// The files that this function has updated
	pub files_updated: HashSet<PathBuf>,
}

impl UpdateMethodResult {
	/// Create a new UpdateMethodResult
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a new UpdateMethodResult from one path
	pub fn from_path(path: PathBuf) -> Self {
		let mut out = Self::new();
		out.files_updated.insert(path);
		out
	}

	/// Merges this result with another one
	pub fn merge(&mut self, other: Self) {
		self.files_updated.extend(other.files_updated);
	}
}
