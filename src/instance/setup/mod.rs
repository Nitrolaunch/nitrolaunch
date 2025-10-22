/// Creation of the client
mod client;
/// Creation of the server
mod server;

use std::collections::HashSet;
use std::fs;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use nitro_config::instance::QuickPlay;
use nitro_core::instance::WindowResolution;
use nitro_core::io::java::classpath::Classpath;
use nitro_core::io::json_to_file;
use nitro_core::launch::LaunchConfiguration;
use nitro_core::user::uuid::hyphenate_uuid;
use nitro_core::user::{User, UserManager};
use nitro_core::version::InstalledVersion;
use nitro_core::QuickPlayType;
use nitro_plugin::hooks::{OnInstanceSetup, OnInstanceSetupArg, RemoveLoader};
use nitro_shared::output::OutputProcess;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::translate;
use nitro_shared::Side;

use crate::io::lock::Lockfile;
use crate::io::paths::Paths;
use crate::plugin::PluginManager;

use super::update::manager::{UpdateManager, UpdateMethodResult, UpdateRequirement};
use super::{InstKind, Instance};

/// The default main class for the server
pub const DEFAULT_SERVER_MAIN_CLASS: &str = "net.minecraft.server.Main";

impl Instance {
	/// Get the requirements for this instance
	pub fn get_requirements(&self) -> HashSet<UpdateRequirement> {
		let mut out = HashSet::new();
		match &self.kind {
			InstKind::Client { .. } => {
				if self.config.launch.use_log4j_config {
					out.insert(UpdateRequirement::ClientLoggingConfig);
				}
			}
			InstKind::Server { .. } => {}
		}
		out
	}

	/// Setup the data and folders for the instance, preparing it for launch
	pub async fn setup(
		&mut self,
		manager: &mut UpdateManager,
		plugins: &PluginManager,
		paths: &Paths,
		users: &UserManager,
		lock: &mut Lockfile,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<UpdateMethodResult> {
		// Start by setting up side-specific stuff
		let result = match &self.kind {
			InstKind::Client { .. } => {
				o.display(
					MessageContents::Header(translate!(o, StartUpdatingClient)),
					MessageLevel::Important,
				);
				o.start_section();
				let result = self
					.setup_client(paths, users)
					.await
					.context("Failed to create client")?;
				Ok::<_, anyhow::Error>(result)
			}
			InstKind::Server { .. } => {
				o.display(
					MessageContents::Header(translate!(o, StartUpdatingServer)),
					MessageLevel::Important,
				);
				o.start_section();
				let result = self
					.setup_server(paths)
					.await
					.context("Failed to create server")?;
				Ok(result)
			}
		}?;

		self.ensure_dirs(paths)?;

		let update_depth = manager.settings.depth;
		let version_info = manager.version_info.get_clone();

		// Get the Java installation and game JAR ahead of time for plugins to use

		let mut version = manager
			.get_core_version(o)
			.await
			.context("Failed to get manager version")?;

		let jvm_path = version
			.get_java_installation(self.config.launch.java.clone(), o)
			.await?
			.get_jvm_path();

		let game_jar_path = version.get_game_jar(self.get_side(), o).await?;

		// Run plugin setup hooks

		lock.ensure_instance_created(&self.id, &version_info.version);
		let lock_instance = lock.get_instance(&self.id);
		let current_version = lock_instance.map(|x| x.version.clone());
		let current_loader_version = lock_instance.and_then(|x| x.loader_version.clone());
		let current_loader = lock_instance.map(|x| x.loader.clone()).unwrap_or_default();

		let mut arg = OnInstanceSetupArg {
			id: self.id.to_string(),
			side: Some(self.get_side()),
			game_dir: self.dirs.get().game_dir.to_string_lossy().to_string(),
			version_info: version_info.clone(),
			loader: self.config.loader.clone(),
			current_loader_version,
			desired_loader_version: self.config.loader_version.clone(),
			config: self
				.config
				.original_config_with_profiles_and_plugins
				.clone(),
			internal_dir: paths.internal.to_string_lossy().to_string(),
			update_depth,
			jvm_path: jvm_path.to_string_lossy().to_string(),
			game_jar_path: game_jar_path.to_string_lossy().to_string(),
		};

		// Do loader and version change checks
		let is_version_different = current_version
			.as_ref()
			.is_some_and(|x| x != &version_info.version);
		let is_loader_different = self.config.loader != current_loader;

		if is_version_different || is_loader_different {
			let mut process = OutputProcess::new(o);
			if is_loader_different {
				let message =
					MessageContents::StartProcess(translate!(process, StartUpdatingInstanceLoader));
				process.display(message, MessageLevel::Important);
			} else if is_version_different {
				let message = MessageContents::StartProcess(translate!(
					process,
					StartUpdatingInstanceVersion,
					"version1" = &current_version.as_ref().expect("Version should exist"),
					"version2" = &version_info.version
				));
				process.display(message, MessageLevel::Important);
			}

			// Teardown
			self.teardown(paths)
				.context("Failed to teardown instance")?;

			arg.loader = current_loader;
			if let Some(current_version) = &current_version {
				arg.version_info.version = current_version.clone();
			}
			let results = plugins
				.call_hook(RemoveLoader, &arg, paths, process.deref_mut())
				.await
				.context("Failed to call remove loader hook")?;

			for result in results {
				result.result(process.deref_mut()).await?;
			}

			// The current loader version is no longer valid as it is referring to the old loader
			arg.current_loader_version = None;
			arg.loader = self.config.loader.clone();
			arg.version_info.version = version_info.version.clone();

			let message =
				MessageContents::Success(translate!(process, FinishUpdatingInstanceVersion));
			process.display(message, MessageLevel::Important);
		}

		let results = plugins
			.call_hook(OnInstanceSetup, &arg, paths, o)
			.await
			.context("Failed to call instance setup hook")?;

		let mut loader_version_set = false;
		for result in results {
			let result = result.result(o).await?;
			self.modification_data
				.classpath_extension
				.add_multiple(result.classpath_extension.iter());

			if let Some(main_class) = result.main_class_override {
				if self.modification_data.main_class_override.is_none() {
					self.modification_data.main_class_override = Some(main_class);
				} else {
					bail!("Multiple plugins overwrote the main class");
				}
			}

			if let Some(jar_path) = result.jar_path_override {
				if self.modification_data.jar_path_override.is_none() {
					self.modification_data.jar_path_override = Some(PathBuf::from(jar_path));
				} else {
					bail!("Multiple plugins overwrote the JAR path");
				}
			}

			self.modification_data.jvm_args.extend(result.jvm_args);
			self.modification_data.game_args.extend(result.game_args);

			if let Some(loader_version) = result.loader_version {
				if loader_version_set {
					bail!("Multiple plugins attempted to modify the loader version");
				}
				lock.update_instance_loader_version(&self.id, Some(loader_version))
					.expect("Instance should exist");
				loader_version_set = true;
			}
		}

		// Update the loaders and version
		lock.update_instance_version(&self.id, &version_info.version)
			.expect("Instance should exist");
		lock.update_instance_loader(&self.id, self.config.loader.clone())
			.expect("Instance should exist");

		lock.finish(paths)
			.context("Failed to finish using lockfile")?;

		self.create_core_instance(&mut version, paths, o)
			.await
			.context("Failed to create core instance")?;

		o.end_section();

		Ok(result)
	}

	/// Ensure the directories are set and exist
	pub fn ensure_dirs(&mut self, paths: &Paths) -> anyhow::Result<()> {
		self.dirs.ensure_full(|| {
			InstanceDirs::new(
				paths,
				&self.id,
				&self.kind.to_side(),
				self.config.game_dir.as_deref(),
			)
		});
		self.dirs.get().ensure_exist()?;

		Ok(())
	}

	/// Create the core instance
	pub(super) async fn create_core_instance<'core>(
		&mut self,
		version: &'core mut InstalledVersion<'core, 'core>,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<nitro_core::Instance<'core>> {
		self.ensure_dirs(paths)?;
		let side = match &self.kind {
			InstKind::Client { window, .. } => nitro_core::InstanceKind::Client {
				window: nitro_core::ClientWindowConfig {
					resolution: window
						.resolution
						.map(|x| WindowResolution::new(x.width, x.height)),
				},
			},
			InstKind::Server { .. } => nitro_core::InstanceKind::Server {
				create_eula: true,
				show_gui: false,
			},
		};
		let quick_play = match self.config.launch.quick_play.clone() {
			QuickPlay::None => QuickPlayType::None,
			QuickPlay::Server { server, port } => QuickPlayType::Server { server, port },
			QuickPlay::World { world } => QuickPlayType::World { world },
			QuickPlay::Realm { realm } => QuickPlayType::Realm { realm },
		};
		let wrapper = self
			.config
			.launch
			.wrapper
			.as_ref()
			.map(|x| nitro_core::WrapperCommand {
				cmd: x.cmd.clone(),
				args: x.args.clone(),
			});

		let mut jvm_args = self.config.launch.jvm_args.clone();
		jvm_args.extend(self.modification_data.jvm_args.clone());

		let mut game_args = self.config.launch.game_args.clone();
		game_args.extend(self.modification_data.game_args.clone());

		let launch_config = LaunchConfiguration {
			java: self.config.launch.java.clone(),
			jvm_args,
			game_args,
			min_mem: self.config.launch.min_mem.clone(),
			max_mem: self.config.launch.max_mem.clone(),
			env: self.config.launch.env.clone(),
			wrappers: Vec::from_iter(wrapper),
			quick_play,
			use_log4j_config: self.config.launch.use_log4j_config,
		};
		let config = nitro_core::InstanceConfiguration {
			side,
			path: self.dirs.get().game_dir.clone(),
			launch: launch_config,
			jar_path: self.modification_data.jar_path_override.clone(),
			main_class: self.modification_data.main_class_override.clone(),
			additional_libs: self.modification_data.classpath_extension.get_paths(),
		};
		let inst = version
			.get_instance(config, o)
			.await
			.context("Failed to initialize instance")?;
		Ok(inst)
	}

	/// Removes files such as the game jar for when the profile version changes
	pub fn teardown(&mut self, paths: &Paths) -> anyhow::Result<()> {
		self.ensure_dirs(paths)?;
		match self.kind {
			InstKind::Client { .. } => {
				let inst_dir = &self.dirs.get().inst_dir;
				let jar_path = inst_dir.join("client.jar");
				if jar_path.exists() {
					fs::remove_file(jar_path).context("Failed to remove client.jar")?;
				}
			}
			InstKind::Server { .. } => {
				let game_dir = &self.dirs.get().game_dir;
				let jar_path = game_dir.join("server.jar");
				if jar_path.exists() {
					fs::remove_file(jar_path).context("Failed to remove server.jar")?;
				}
			}
		}

		Ok(())
	}

	/// Removes all game files for an instance, including saves. Be wary when using this!
	pub async fn delete_files(&mut self, paths: &Paths) -> anyhow::Result<()> {
		self.ensure_dirs(paths)?;
		tokio::fs::remove_dir_all(&self.dirs.get().inst_dir).await?;

		Ok(())
	}

	/// Create a keypair file in the instance
	fn create_keypair(&mut self, user: &User, paths: &Paths) -> anyhow::Result<()> {
		if let Some(uuid) = user.get_uuid() {
			if let Some(keypair) = user.get_keypair() {
				self.ensure_dirs(paths)?;
				let keys_dir = self.dirs.get().game_dir.join("profilekeys");
				let hyphenated_uuid = hyphenate_uuid(uuid).context("Failed to hyphenate UUID")?;
				let path = keys_dir.join(format!("{hyphenated_uuid}.json"));
				nitro_core::io::files::create_leading_dirs(&path)?;

				json_to_file(path, keypair).context("Failed to write keypair to file")?;
			}
		}

		Ok(())
	}
}

/// Directories that an instance uses
#[derive(Debug)]
pub struct InstanceDirs {
	/// The base instance directory
	pub inst_dir: PathBuf,
	/// The game directory, such as .minecraft, relative to the instance directory
	pub game_dir: PathBuf,
}

impl InstanceDirs {
	/// Create a new InstanceDirs
	pub fn new(
		paths: &Paths,
		instance_id: &str,
		side: &Side,
		game_dir_override: Option<&Path>,
	) -> Self {
		let inst_dir = paths.data.join("instances").join(instance_id);

		let game_dir = if let Some(game_dir) = game_dir_override {
			game_dir.to_owned()
		} else {
			match side {
				Side::Client => inst_dir.join(".minecraft"),
				Side::Server => inst_dir.clone(),
			}
		};

		Self { inst_dir, game_dir }
	}

	/// Make sure the directories exist
	pub fn ensure_exist(&self) -> anyhow::Result<()> {
		std::fs::create_dir_all(&self.inst_dir).context("Failed to create instance directory")?;
		std::fs::create_dir_all(&self.game_dir)
			.context("Failed to create instance game directory")?;
		Ok(())
	}
}

/// Things that modifications for an instance change when creating it
#[derive(Debug)]
pub struct ModificationData {
	/// Override for the main class from modifications
	pub main_class_override: Option<String>,
	/// Override for the Jar path from modifications
	pub jar_path_override: Option<PathBuf>,
	/// Extension for the classpath from modifications
	pub classpath_extension: Classpath,
	/// Extra arguments for the JVM
	pub jvm_args: Vec<String>,
	/// Extra arguments for the game
	pub game_args: Vec<String>,
}

impl ModificationData {
	/// Create a new ModificationData with default parameters
	pub fn new() -> Self {
		Self {
			main_class_override: None,
			jar_path_override: None,
			classpath_extension: Classpath::new(),
			jvm_args: Vec::new(),
			game_args: Vec::new(),
		}
	}
}

impl Default for ModificationData {
	fn default() -> Self {
		Self::new()
	}
}
