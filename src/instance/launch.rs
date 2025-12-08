use nitro_core::io::files::open_file_append;
use nitro_core::launch::get_stdio_file_path;
use nitro_core::NitroCore;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;
use sysinfo::{Pid, System};

use anyhow::{bail, Context};
use nitro_config::instance::{QuickPlay, WrapperCommand};
use nitro_core::auth_crate::mc::ClientId;
use nitro_core::io::java::install::JavaInstallationKind;
use nitro_core::user::UserManager;
use nitro_plugin::hook::call::HookHandles;
use nitro_plugin::hook::hooks::{
	InstanceLaunchArg, OnInstanceLaunch, OnInstanceStop, ReplaceInstanceLaunch, WhileInstanceLaunch,
};
use nitro_shared::id::InstanceID;
use nitro_shared::java_args::MemoryNum;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::{translate, Side, UpdateDepth};
use reqwest::Client;
use tokio::io::{AsyncWriteExt, Stdout};

use super::tracking::RunningInstanceRegistry;
use super::update::manager::UpdateManager;
use crate::instance::setup::setup_core;
use crate::instance::tracking::is_process_alive;
use crate::instance::update::manager::UpdateSettings;
use crate::instance::world_files::WorldFilesWatcher;
use crate::io::lock::Lockfile;
use crate::io::paths::Paths;
use crate::plugin::PluginManager;

use super::Instance;

impl Instance {
	/// Launch the instance process
	pub async fn launch(
		&mut self,
		paths: &Paths,
		users: &mut UserManager,
		plugins: &PluginManager,
		settings: LaunchSettings,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<InstanceHandle> {
		o.display(
			MessageContents::StartProcess(translate!(o, StartUpdatingInstance, "inst" = &self.id)),
			MessageLevel::Important,
		);

		let mut manager = UpdateManager::from_settings(UpdateSettings {
			depth: UpdateDepth::Shallow,
			offline_auth: settings.offline_auth,
		});
		let client = Client::new();

		let mut core = setup_core(
			Some(&settings.ms_client_id),
			&manager.settings,
			&client,
			users,
			plugins,
			paths,
			o,
		)
		.await
		.context("Failed to configure core")?;

		let core_version = core.get_version(&self.config.version, o).await?;
		let version_info = core_version.get_version_info();
		std::mem::drop(core_version);

		let mut lock = Lockfile::open(paths).context("Failed to open lockfile")?;
		let result = self
			.setup(
				&mut manager,
				&mut core,
				&version_info,
				plugins,
				paths,
				users,
				&mut lock,
				o,
			)
			.await
			.context("Failed to update instance")?;
		manager.add_result(result);

		let hook_arg = InstanceLaunchArg {
			id: self.id.to_string(),
			side: Some(self.get_side()),
			dir: self.dirs.get().inst_dir.to_string_lossy().into(),
			game_dir: self
				.dirs
				.get()
				.game_dir
				.as_ref()
				.map(|x| x.to_string_lossy().into()),
			version_info: version_info.clone(),
			config: self
				.config
				.original_config_with_templates_and_plugins
				.clone(),
			pid: None,
			stdout_path: None,
			stdin_path: None,
		};

		// Make sure that any fluff from the update gets ended
		o.end_process();

		o.display(
			MessageContents::StartProcess(translate!(o, PreparingLaunch)),
			MessageLevel::Important,
		);

		// Run pre-launch hooks
		let results = plugins
			.call_hook(OnInstanceLaunch, &hook_arg, paths, o)
			.await
			.context("Failed to call on launch hook")?;
		results.all_results(o).await?;

		if self.dirs.get().game_dir.is_some() && !self.config.custom_launch {
			self.launch_standard(core, hook_arg, paths, plugins, settings, o)
				.await
		} else {
			self.launch_custom(hook_arg, paths, plugins, o).await
		}
	}

	/// Standard Java launch
	async fn launch_standard(
		&mut self,
		mut core: NitroCore,
		mut hook_arg: InstanceLaunchArg,
		paths: &Paths,
		plugins: &PluginManager,
		settings: LaunchSettings,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<InstanceHandle> {
		let mut core_version = core.get_version(&self.config.version, o).await?;

		let mut instance = self
			.create_core_instance(&mut core_version, paths, o)
			.await
			.context("Failed to create core instance")?;

		instance.pipe_stdin(settings.pipe_stdin);

		// Launch the instance using core
		let handle = instance
			.launch_with_handle(o)
			.await
			.context("Failed to launch core instance")?;

		hook_arg.pid = Some(handle.get_pid());
		hook_arg.stdout_path = Some(handle.stdout_path().to_string_lossy().to_string());
		hook_arg.stdin_path = handle.stdin_path().map(|x| x.to_string_lossy().to_string());

		// Run while_instance_launch hooks alongside
		let hook_handles = plugins
			.call_hook(WhileInstanceLaunch, &hook_arg, paths, o)
			.await
			.context("Failed to call while launch hook")?;

		let world_files = if self.get_side() == Side::Client {
			Some(
				WorldFilesWatcher::new(self.dirs.get().game_dir.as_ref().unwrap(), plugins.clone())
					.context("Failed to setup world files watcher")?,
			)
		} else {
			None
		};

		let handle = InstanceHandle {
			instance_id: self.id.clone(),
			hook_handles,
			hook_arg,
			stdout: tokio::io::stdout(),
			is_silent: false,
			inner: InstanceHandleInner::Standard {
				inner: handle,
				world_files,
			},
		};

		// Update the running instance registry
		let mut running_instance_registry = RunningInstanceRegistry::open(paths)
			.context("Failed to open registry of running instances")?;
		running_instance_registry.add_instance(handle.get_pid(), &self.id, true);
		let _ = running_instance_registry.write();

		Ok(handle)
	}

	/// Custom launch using a plugin
	async fn launch_custom(
		&mut self,
		mut hook_arg: InstanceLaunchArg,
		paths: &Paths,
		plugins: &PluginManager,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<InstanceHandle> {
		// Set up stdio
		let stdout_path = get_stdio_file_path(&paths.core, false);
		let stdin_path = get_stdio_file_path(&paths.core, true);
		hook_arg.stdout_path = Some(stdout_path.to_string_lossy().to_string());
		hook_arg.stdin_path = Some(stdin_path.to_string_lossy().to_string());

		let result = plugins
			.call_hook(ReplaceInstanceLaunch, &hook_arg, paths, o)
			.await
			.context("Failed to call custom launch hook")?;

		let Some(result) = result.first_some(o).await? else {
			bail!("No plugins handled custom launch for this instance");
		};

		hook_arg.pid = Some(result.pid);

		// Run while_instance_launch hooks alongside
		let hook_handles = plugins
			.call_hook(WhileInstanceLaunch, &hook_arg, paths, o)
			.await
			.context("Failed to call while launch hook")?;

		let stdout_file = File::open(&stdout_path)?;
		let stdin_file = open_file_append(&stdin_path)?;

		let handle = InstanceHandle {
			instance_id: self.id.clone(),
			hook_handles,
			hook_arg,
			stdout: tokio::io::stdout(),
			is_silent: false,
			inner: InstanceHandleInner::Plugin {
				pid: result.pid,
				stdout_file,
				stdin_file,
				stdout_path,
				stdin_path,
			},
		};

		// Update the running instance registry
		let mut running_instance_registry = RunningInstanceRegistry::open(paths)
			.context("Failed to open registry of running instances")?;
		running_instance_registry.add_instance(handle.get_pid(), &self.id, true);
		let _ = running_instance_registry.write();

		Ok(handle)
	}
}

/// Settings for launch provided to the instance launch function
pub struct LaunchSettings {
	/// The Microsoft client ID to use
	pub ms_client_id: ClientId,
	/// Whether to do offline auth
	pub offline_auth: bool,
	/// Whether to pipe the stdin of this process into the instance process
	pub pipe_stdin: bool,
}

/// Options for launching after conversion from the deserialized version
#[derive(Debug)]
pub struct LaunchOptions {
	/// Java kind
	pub java: JavaInstallationKind,
	/// JVM arguments
	pub jvm_args: Vec<String>,
	/// Game arguments
	pub game_args: Vec<String>,
	/// Minimum JVM memory
	pub min_mem: Option<MemoryNum>,
	/// Maximum JVM memory
	pub max_mem: Option<MemoryNum>,
	/// Environment variables
	pub env: HashMap<String, String>,
	/// Wrapper command
	pub wrapper: Option<WrapperCommand>,
	/// Quick Play options
	pub quick_play: QuickPlay,
	/// Whether or not to use the Log4J configuration
	pub use_log4j_config: bool,
}

/// A handle for an instance
pub struct InstanceHandle {
	/// The ID of the instance
	instance_id: InstanceID,
	/// Handles for hooks running while the instance is running
	hook_handles: HookHandles<WhileInstanceLaunch>,
	/// Arg to pass to the stop hook when the instance is stopped
	hook_arg: InstanceLaunchArg,
	/// Global stdout
	stdout: Stdout,
	/// Whether to redirect stdin and stdout to the process stdin and stdout
	is_silent: bool,
	/// Inner implementation
	inner: InstanceHandleInner,
}

/// Inner implementation for an InstanceHandle,
/// depending on whether this is a normal launch or a plugin launch
enum InstanceHandleInner {
	Standard {
		/// Core InstanceHandle with the process
		inner: nitro_core::InstanceHandle,
		/// Shared world file watcher
		world_files: Option<WorldFilesWatcher>,
	},
	Plugin {
		/// PID of the instance process
		pid: u32,
		/// Stdout file for the process
		stdout_file: File,
		/// Stdin file for the process
		stdin_file: File,
		/// Stdout file path
		stdout_path: PathBuf,
		/// Stdin file path
		stdin_path: PathBuf,
	},
}

impl InstanceHandle {
	/// Waits for the process to complete
	pub async fn wait(
		mut self,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<std::process::ExitStatus> {
		let pid = self.get_pid();
		let mut system = System::new_all();

		// Wait for the process to complete while polling plugins and stdio
		let mut stdio_buf = [0u8; 512];
		let status = loop {
			// Plugins
			let _ = self.hook_handles.poll_all(o).await;

			// Instance stdio
			if !self.is_silent {
				let inst_stdout = match &mut self.inner {
					InstanceHandleInner::Standard { inner, .. } => inner.stdout(),
					InstanceHandleInner::Plugin { stdout_file, .. } => stdout_file,
				};

				// This is non-blocking as the stdout file will have an EoF
				if let Ok(bytes_read) = inst_stdout.read(&mut stdio_buf) {
					let _ = self.stdout.write(&stdio_buf[0..bytes_read]).await;
				}
			}

			// Update world files
			if let InstanceHandleInner::Standard { world_files, .. } = &mut self.inner {
				if let Some(world_files) = world_files {
					let _ = world_files.watch(&self.hook_arg, paths, o).await;
				}
			}

			// Check if the instance has exited
			match &mut self.inner {
				InstanceHandleInner::Standard { inner, .. } => {
					if let Ok(Some(status)) = inner.try_wait() {
						break status;
					}
				}
				InstanceHandleInner::Plugin { pid, .. } => {
					system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
					if !is_process_alive(*pid, &system, false) {
						break ExitStatus::default();
					}
				}
			}

			tokio::time::sleep(Duration::from_millis(5)).await;
		};

		// Terminate any sibling processes now that the main one is complete
		self.hook_handles.terminate().await;

		Self::on_stop(&self.instance_id, pid, &self.hook_arg, plugins, paths, o).await?;

		Ok(status)
	}

	/// Kills the process early
	pub async fn kill(
		self,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let pid = self.get_pid();

		let _ = self.hook_handles.kill(o).await;
		match self.inner {
			InstanceHandleInner::Standard { mut inner, .. } => {
				let _ = inner.kill();
			}
			InstanceHandleInner::Plugin { pid, .. } => {
				let mut system = System::new_all();
				system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
				if let Some(proc) = system.process(Pid::from(pid as usize)) {
					proc.kill();
				}
			}
		}

		Self::on_stop(&self.instance_id, pid, &self.hook_arg, plugins, paths, o).await?;

		Ok(())
	}

	/// Gets the internal child process for the game, consuming the
	/// InstanceHandle
	pub fn get_process(self) -> Option<std::process::Child> {
		match self.inner {
			InstanceHandleInner::Standard { inner, .. } => Some(inner.get_process()),
			InstanceHandleInner::Plugin { .. } => None,
		}
	}

	/// Gets the PID of the instance process
	pub fn get_pid(&self) -> u32 {
		match &self.inner {
			InstanceHandleInner::Standard { inner, .. } => inner.get_pid(),
			InstanceHandleInner::Plugin { pid, .. } => *pid,
		}
	}

	/// Set whether the stdio of the instance should be redirected to this process
	pub fn silence_output(&mut self, is_silent: bool) {
		self.is_silent = is_silent;
	}

	/// Get the stdout file path for this instance
	pub fn stdout(&self) -> &Path {
		match &self.inner {
			InstanceHandleInner::Standard { inner, .. } => inner.stdout_path(),
			InstanceHandleInner::Plugin { stdout_path, .. } => stdout_path,
		}
	}

	/// Get the stdin file path for this instance
	pub fn stdin(&self) -> Option<&Path> {
		match &self.inner {
			InstanceHandleInner::Standard { inner, .. } => inner.stdin_path(),
			InstanceHandleInner::Plugin { stdin_path, .. } => Some(stdin_path),
		}
	}

	/// Writes to stdin
	pub fn write_stdin(&mut self, data: &[u8]) -> anyhow::Result<()> {
		match &mut self.inner {
			InstanceHandleInner::Standard { inner, .. } => inner
				.write_stdin(data)
				.context("Failed to write to inner stdin"),
			InstanceHandleInner::Plugin { stdin_file, .. } => stdin_file
				.write_all(data)
				.context("Failed to write to stdin file"),
		}
	}

	/// Function that should be run whenever the instance stops
	async fn on_stop(
		instance_id: &str,
		pid: u32,
		arg: &InstanceLaunchArg,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		// Remove the instance from the registry
		let running_instance_registry = RunningInstanceRegistry::open(paths);
		if let Ok(mut running_instance_registry) = running_instance_registry {
			running_instance_registry.remove_instance(pid, instance_id);
			let _ = running_instance_registry.write();
		}

		// Call on stop hooks
		let results = plugins
			.call_hook(OnInstanceStop, arg, paths, o)
			.await
			.context("Failed to call on stop hook")?;
		results.all_results(o).await?;

		Ok(())
	}
}
