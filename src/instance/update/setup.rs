use std::fs;
use std::ops::DerefMut;
use std::path::PathBuf;

use anyhow::{Context, bail};
use nitro_config::instance::{QuickPlay, WrapperCommand};
use nitro_core::account::Account;
use nitro_core::instance::WindowResolution;
use nitro_core::io::java::classpath::Classpath;
use nitro_core::io::json_to_file;
use nitro_core::launch::LaunchConfiguration;
use nitro_core::version::InstalledVersion;
use nitro_core::{NitroCore, QuickPlayType};
use nitro_instance::lock::InstanceLockfile;
use nitro_plugin::hook::hooks::{
	AfterInstanceSetup, OnInstanceSetup, OnInstanceSetupArg, OnInstanceSetupResult, RemoveLoader,
};
use nitro_shared::Side;
use nitro_shared::output::OutputProcess;
use nitro_shared::output::{MessageContents, NitroOutput};
use nitro_shared::translate;
use nitro_shared::uuid::hyphenate_uuid;
use nitro_shared::versions::VersionInfo;

use crate::io::paths::Paths;
use crate::plugin::PluginManager;

use super::super::{InstKind, Instance};
use super::manager::UpdateManager;

/// The default main class for the server
pub const DEFAULT_SERVER_MAIN_CLASS: &str = "net.minecraft.server.Main";

impl Instance {
	/// Setup the data and folders for the instance, preparing it for launch
	pub async fn setup(
		&mut self,
		manager: &mut UpdateManager,
		core: &NitroCore,
		version_info: &VersionInfo,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		self.ensure_dir()?;

		let update_depth = manager.settings.depth;

		// Get the Java installation and game JAR ahead of time for plugins to use

		let mut version = core
			.get_version(&self.version, manager.settings.depth, o)
			.await?;

		let jvm_path = version
			.get_java_installation(self.launch.java.clone(), o)
			.await?
			.get_jvm_path();

		let game_jar_path = version.get_game_jar(self.side(), o).await?;

		// Run plugin setup hooks

		let mut inst_lock = self
			.get_lockfile(paths)
			.context("Failed to open instance lockfile")?;

		let current_version = inst_lock.get_minecraft_version().cloned();
		let current_loader = inst_lock.get_loader().clone();
		let current_loader_version = inst_lock.get_loader_version().cloned();

		let mut arg = OnInstanceSetupArg {
			id: self.id.to_string(),
			side: Some(self.side()),
			inst_dir: self.dir.as_ref().map(|x| x.to_string_lossy().to_string()),
			version_info: version_info.clone(),
			old_version: current_version.clone(),
			loader: self.loader.clone(),
			current_loader_version,
			desired_loader_version: self.loader_version.clone(),
			config: self.config.clone(),
			internal_dir: paths.internal.to_string_lossy().to_string(),
			update_depth,
			jvm_path: jvm_path.to_string_lossy().to_string(),
			game_jar_path: game_jar_path.to_string_lossy().to_string(),
			classpath: None,
		};

		// Do loader and version change checks
		let is_version_different = current_version
			.as_ref()
			.is_some_and(|x| x != &version_info.version);
		let is_loader_different = self.loader != current_loader;

		if is_version_different || is_loader_different {
			let mut process = OutputProcess::new(o);
			if is_loader_different {
				let message =
					MessageContents::StartProcess(translate!(process, StartUpdatingInstanceLoader));
				process.display(message);
			} else if is_version_different {
				let message = MessageContents::StartProcess(translate!(
					process,
					StartUpdatingInstanceVersion,
					"version1" = &current_version.as_ref().expect("Version should exist"),
					"version2" = &version_info.version
				));
				process.display(message);
			}

			// Teardown
			self.teardown().context("Failed to teardown instance")?;

			arg.loader = current_loader;
			if let Some(current_version) = &current_version {
				arg.version_info.version = current_version.clone();
			}

			let results = plugins
				.call_hook(RemoveLoader, &arg, paths, process.deref_mut())
				.await
				.context("Failed to call remove loader hook")?;
			results.all_results(process.deref_mut()).await?;

			// The current loader version is no longer valid as it is referring to the old loader
			arg.current_loader_version = None;
			arg.loader = self.loader.clone();
			arg.version_info.version = version_info.version.clone();

			let message =
				MessageContents::Success(translate!(process, FinishUpdatingInstanceVersion));
			process.display(message);
		}

		let mut results = plugins
			.call_hook(OnInstanceSetup, &arg, paths, o)
			.await
			.context("Failed to call instance setup hook")?;

		while let Some(result) = results.next_result(o).await? {
			self.modify_from_setup_result(result, &mut inst_lock)?;
		}

		// Update the loaders and version
		inst_lock.update_minecraft_version(&version_info.version);
		inst_lock.update_loader(self.loader.clone());

		inst_lock
			.write()
			.context("Failed to finish using lockfile")?;

		// Create the core instance only if this is a local instance
		if self.dir.is_some() {
			let instance = self
				.create_core_instance(&mut version, paths, o)
				.await
				.context("Failed to create core instance")?;
			arg.classpath = Some(instance.get_classpath().get_str());

			let mut results = plugins
				.call_hook(AfterInstanceSetup, &arg, paths, o)
				.await
				.context("Failed to call after instance setup hook")?;
			while let Some(result) = results.next_result(o).await? {
				self.modify_from_setup_result(result, &mut inst_lock)?;
			}
		}

		Ok(())
	}

	fn modify_from_setup_result(
		&mut self,
		result: OnInstanceSetupResult,
		lock: &mut InstanceLockfile,
	) -> anyhow::Result<()> {
		self.modification_data
			.classpath_extension
			.add_multiple(result.classpath_extension.iter());

		if let Some(main_class) = result.main_class_override {
			self.modification_data.main_class_override = Some(main_class);
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
		self.modification_data.wrappers.extend(result.wrappers);

		self.modification_data.exclude_game_jar |= result.exclude_game_jar;

		if let Some(loader_version) = result.loader_version {
			lock.update_loader_version(Some(loader_version));
		}

		Ok(())
	}

	/// Ensure the instance directory exists
	pub fn ensure_dir(&self) -> anyhow::Result<()> {
		if let Some(dir) = &self.dir {
			std::fs::create_dir_all(dir)?;
		}

		Ok(())
	}

	/// Create the core instance
	pub(crate) async fn create_core_instance(
		&mut self,
		version: &InstalledVersion,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<nitro_core::Instance> {
		self.ensure_dir()?;
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
		let mut wrappers = Vec::from_iter(wrapper);
		wrappers.extend(self.modification_data.wrappers.iter().map(|x| {
			nitro_core::WrapperCommand {
				cmd: x.cmd.clone(),
				args: x.args.clone(),
			}
		}));

		let mut jvm_args = self.launch.jvm_args.clone();
		jvm_args.extend(self.modification_data.jvm_args.clone());

		let mut game_args = self.launch.game_args.clone();
		game_args.extend(self.modification_data.game_args.clone());

		let launch_config = LaunchConfiguration {
			java: self.launch.java.clone(),
			jvm_args,
			game_args,
			min_mem: self.launch.min_mem.clone(),
			max_mem: self.launch.max_mem.clone(),
			env: self.launch.env.clone(),
			wrappers,
			quick_play,
			use_log4j_config: self.launch.use_log4j_config,
		};
		let inst_dir = self
			.dir
			.clone()
			.unwrap_or(paths.data.join("instances").join(&*self.id));
		let config = nitro_core::InstanceConfiguration {
			side,
			path: inst_dir,
			launch: launch_config,
			jar_path: self.modification_data.jar_path_override.clone(),
			main_class: self.modification_data.main_class_override.clone(),
			additional_libs: self.modification_data.classpath_extension.get_paths(),
			exclude_game_jar: self.modification_data.exclude_game_jar,
		};
		let inst = version
			.get_instance(config, o)
			.await
			.context("Failed to initialize instance")?;
		Ok(inst)
	}

	/// Removes files such as the game jar for when the template version changes
	pub fn teardown(&mut self) -> anyhow::Result<()> {
		if let Some(inst_dir) = &self.dir {
			match self.kind {
				InstKind::Client { .. } => {
					let jar_path = inst_dir.join("client.jar");
					if jar_path.exists() {
						fs::remove_file(jar_path).context("Failed to remove client.jar")?;
					}
				}
				InstKind::Server { .. } => {
					let jar_path = inst_dir.join("server.jar");
					if jar_path.exists() {
						fs::remove_file(jar_path).context("Failed to remove server.jar")?;
					}
				}
			}
		}

		Ok(())
	}

	/// Create a keypair file in the instance
	pub fn create_keypair(&mut self, account: &Account) -> anyhow::Result<()> {
		if self.side() != Side::Client {
			return Ok(());
		}

		if let Some(uuid) = account.get_uuid() {
			if let Some(keypair) = account.get_keypair() {
				self.ensure_dir()?;
				if let Some(inst_dir) = &self.dir {
					let keys_dir = inst_dir.join("profilekeys");
					let hyphenated_uuid =
						hyphenate_uuid(uuid).context("Failed to hyphenate UUID")?;
					let path = keys_dir.join(format!("{hyphenated_uuid}.json"));
					nitro_core::io::files::create_leading_dirs(&path)?;

					json_to_file(path, keypair).context("Failed to write keypair to file")?;
				}
			}
		}

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
	/// Extra wrapper commands for the game
	pub wrappers: Vec<WrapperCommand>,
	/// Whether to skip adding the game JAR to the classpath
	pub exclude_game_jar: bool,
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
			wrappers: Vec::new(),
			exclude_game_jar: false,
		}
	}
}

impl Default for ModificationData {
	fn default() -> Self {
		Self::new()
	}
}
