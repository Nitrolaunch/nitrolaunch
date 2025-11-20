use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};

use anyhow::Context;
use nitro_config::instance::{QuickPlay, WrapperCommand};
use nitro_core::auth_crate::mc::ClientId;
use nitro_core::io::java::install::JavaInstallationKind;
use nitro_core::user::UserManager;
use nitro_plugin::hook::call::HookHandles;
use nitro_plugin::hook::hooks::{
	InstanceLaunchArg, OnInstanceLaunch, OnInstanceStop, WhileInstanceLaunch,
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

		let mut hook_arg = InstanceLaunchArg {
			id: self.id.to_string(),
			side: Some(self.get_side()),
			dir: self.dirs.get().inst_dir.to_string_lossy().into(),
			game_dir: self.dirs.get().game_dir.to_string_lossy().into(),
			version_info: version_info.clone(),
			config: self
				.config
				.original_config_with_templates_and_plugins
				.clone(),
			pid: None,
			stdout_path: None,
			stdin_path: None,
		};

		let mut core_version = core.get_version(&self.config.version, o).await?;

		let mut instance = self
			.create_core_instance(&mut core_version, paths, o)
			.await
			.context("Failed to create core instance")?;

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

		// Launch the instance using core
		let mut handle = instance
			.launch_with_handle(o)
			.await
			.context("Failed to launch core instance")?;

		hook_arg.pid = Some(handle.get_pid());
		hook_arg.stdout_path = Some(handle.stdout().1.to_string_lossy().to_string());
		hook_arg.stdin_path = Some(handle.stdin().1.to_string_lossy().to_string());

		// Run while_instance_launch hooks alongside
		let hook_handles = plugins
			.call_hook(WhileInstanceLaunch, &hook_arg, paths, o)
			.await
			.context("Failed to call while launch hook")?;

		let world_files = if self.get_side() == Side::Client {
			Some(
				WorldFilesWatcher::new(&self.dirs.get().game_dir, plugins.clone())
					.context("Failed to setup world files watcher")?,
			)
		} else {
			None
		};

		let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(150);

		std::thread::spawn(stdin_thread(stdin_tx));

		let handle = InstanceHandle {
			inner: handle,
			instance_id: self.id.clone(),
			hook_handles,
			hook_arg,
			is_silent: false,
			stdout: tokio::io::stdout(),
			stdin_rx,
			world_files,
		};

		// Update the running instance registry
		let mut running_instance_registry = RunningInstanceRegistry::open(paths)
			.context("Failed to open registry of running instances")?;
		running_instance_registry.add_instance(handle.get_pid(), &self.id);
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
	/// Core InstanceHandle with the process
	inner: nitro_core::InstanceHandle,
	/// The ID of the instance
	instance_id: InstanceID,
	/// Handles for hooks running while the instance is running
	hook_handles: HookHandles<WhileInstanceLaunch>,
	/// Arg to pass to the stop hook when the instance is stopped
	hook_arg: InstanceLaunchArg,
	/// Whether to redirect stdin and stdout to the process stdin and stdout
	is_silent: bool,
	/// Global stdout
	stdout: Stdout,
	/// Receiver for stdin data chunks
	stdin_rx: Receiver<Vec<u8>>,
	/// Shared world file watcher
	world_files: Option<WorldFilesWatcher>,
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

		// Wait for the process to complete while polling plugins and stdio
		let mut stdio_buf = [0u8; 512];
		let status = loop {
			// Plugins
			let _ = self.hook_handles.poll_all(o).await;

			// Instance stdio
			if !self.is_silent {
				let (inst_stdout, _) = self.inner.stdout();
				// This is non-blocking as the stdout file will have an EoF
				if let Ok(bytes_read) = inst_stdout.read(&mut stdio_buf) {
					let _ = self.stdout.write(&stdio_buf[0..bytes_read]).await;
				}

				let (inst_stdin, _) = self.inner.stdin();
				if let Ok(chunk) = self.stdin_rx.try_recv() {
					let _ = inst_stdin.write_all(&chunk);
				}
			}

			// Update world files
			if let Some(world_files) = &mut self.world_files {
				let _ = world_files.watch(&self.hook_arg, paths, o).await;
			}

			// Check if the instance has exited
			let result = self.inner.try_wait();
			if let Ok(Some(status)) = result {
				break status;
			}

			tokio::time::sleep(Duration::from_micros(100)).await;
		};

		// Terminate any sibling processes now that the main one is complete
		self.hook_handles.terminate().await;

		Self::on_stop(&self.instance_id, pid, &self.hook_arg, plugins, paths, o).await?;

		Ok(status)
	}

	/// Kills the process early
	pub async fn kill(
		mut self,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let pid = self.get_pid();

		let _ = self.hook_handles.kill(o).await;
		let _ = self.inner.kill();

		Self::on_stop(&self.instance_id, pid, &self.hook_arg, plugins, paths, o).await?;

		Ok(())
	}

	/// Gets the internal child process for the game, consuming the
	/// InstanceHandle
	pub fn get_process(self) -> std::process::Child {
		self.inner.get_process()
	}

	/// Gets the PID of the instance process
	pub fn get_pid(&self) -> u32 {
		self.inner.get_pid()
	}

	/// Set whether the stdio of the instance should be redirected to this process
	pub fn silence_output(&mut self, is_silent: bool) {
		self.is_silent = is_silent;
	}

	/// Get the stdout file path for this instance
	pub fn stdout(&mut self) -> PathBuf {
		self.inner.stdout().1.clone()
	}

	/// Get the stdin file path for this instance
	pub fn stdin(&mut self) -> PathBuf {
		self.inner.stdin().1.clone()
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

/// Logic for the stdin reader thread
fn stdin_thread(tx: Sender<Vec<u8>>) -> impl Fn() {
	move || {
		let mut stdin = std::io::stdin();
		let mut buffer = [0; 128];
		while !tx.is_closed() {
			if let Ok(bytes_read) = stdin.read(&mut buffer) {
				let chunk = buffer[0..bytes_read].to_vec();
				let _ = tx.blocking_send(chunk);
			}
		}
	}
}
