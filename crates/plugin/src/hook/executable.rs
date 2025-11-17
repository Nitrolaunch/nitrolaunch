use std::{collections::VecDeque, env::consts::EXE_SUFFIX, sync::Arc, time::Instant};

use anyhow::{anyhow, bail, Context};
use nitro_shared::{
	no_window,
	output::{MessageContents, MessageLevel, NitroOutput},
};
use tokio::{
	io::AsyncWriteExt,
	process::{Child, ChildStdin, ChildStdout, Command},
	sync::Mutex,
};

use crate::{
	hook::{
		call::{HookCallArg, HookHandle},
		Hook,
	},
	input_output::{CommandResult, InputAction, OutputAction},
	plugin::PluginPersistence,
	plugin_debug_enabled,
	try_read::TryLineReader,
};

/// The substitution token for the plugin directory in the command
pub static PLUGIN_DIR_TOKEN: &str = "${PLUGIN_DIR}";
/// The substitution token for the executable file extension in the command
pub static EXE_EXTENSION_TOKEN: &str = "${EXE_EXTENSION}";
/// The environment variable for custom config passed to a hook
pub static CUSTOM_CONFIG_ENV: &str = "NITRO_CUSTOM_CONFIG";
/// The environment variable for the data directory passed to a hook
pub static DATA_DIR_ENV: &str = "NITRO_DATA_DIR";
/// The environment variable for the config directory passed to a hook
pub static CONFIG_DIR_ENV: &str = "NITRO_CONFIG_DIR";
/// The environment variable for the plugin state passed to a hook
pub static PLUGIN_STATE_ENV: &str = "NITRO_PLUGIN_STATE";
/// The environment variable for the version of Nitrolaunch
pub static NITRO_VERSION_ENV: &str = "NITRO_VERSION";
/// The environment variable that tells the executable it is running as a plugin
pub static NITRO_PLUGIN_ENV: &str = "NITRO_PLUGIN";
/// The environment variable that tells what version of the hook this is
pub static HOOK_VERSION_ENV: &str = "NITRO_HOOK_VERSION";
/// The environment variable with the list of plugins
pub static PLUGIN_LIST_ENV: &str = "NITRO_PLUGIN_LIST";

/// Calls an executable hook handler
pub(crate) async fn call_executable<H: Hook + Sized>(
	hook: &H,
	arg: HookCallArg<'_, H>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<HookHandle<H>> {
	let _ = o;
	let hook_arg = serde_json::to_string(arg.arg).context("Failed to serialize hook argument")?;

	let plugin_dir = arg
		.working_dir
		.map(|x| x.to_string_lossy().to_string())
		.unwrap_or_default();
	let cmd = arg.cmd.replace(PLUGIN_DIR_TOKEN, &plugin_dir);
	let cmd = cmd.replace(EXE_EXTENSION_TOKEN, EXE_SUFFIX);
	let mut cmd = Command::new(cmd);

	for arg in arg.additional_args {
		cmd.arg(arg.replace(PLUGIN_DIR_TOKEN, &plugin_dir));
	}
	cmd.arg(hook.get_name());
	cmd.arg(hook_arg);

	// Set up environment
	if let Some(custom_config) = arg.custom_config {
		cmd.env(CUSTOM_CONFIG_ENV, custom_config);
	}
	cmd.env(DATA_DIR_ENV, &arg.paths.data);
	cmd.env(CONFIG_DIR_ENV, &arg.paths.config);
	if let Some(nitro_version) = arg.nitro_version {
		cmd.env(NITRO_VERSION_ENV, nitro_version);
	}
	cmd.env(NITRO_PLUGIN_ENV, "1");
	if let Some(working_dir) = arg.working_dir {
		cmd.current_dir(working_dir);
	}
	cmd.env(HOOK_VERSION_ENV, H::get_version().to_string());
	{
		let lock = arg.persistence.lock().await;
		// Don't send null state to improve performance
		if !lock.state.is_null() {
			let state =
				serde_json::to_string(&lock.state).context("Failed to serialize plugin state")?;
			cmd.env(PLUGIN_STATE_ENV, state);
		}
	}
	let plugin_list = arg.plugin_list.join(",");
	cmd.env(PLUGIN_LIST_ENV, plugin_list);

	no_window!(cmd);

	if plugin_debug_enabled() {
		o.display(
			MessageContents::Simple(format!("{cmd:?}")),
			MessageLevel::Important,
		);
	}

	if H::get_takes_over() {
		cmd.spawn()
			.context("Failed to run hook command")?
			.wait()
			.await?;

		Ok(HookHandle::constant(
			H::Result::default(),
			arg.plugin_id.to_string(),
		))
	} else {
		cmd.stdout(std::process::Stdio::piped());
		cmd.stdin(std::process::Stdio::piped());

		let mut child = cmd.spawn()?;

		let stdout = child.stdout.take().unwrap();
		let stdout = TryLineReader::new(stdout);

		let stdin = child.stdin.take().unwrap();

		let start_time = if std::env::var("NITRO_PLUGIN_PROFILE").is_ok_and(|x| x == "1") {
			Some(Instant::now())
		} else {
			None
		};

		let handle_inner = ExecutableHookHandle {
			child,
			stdout,
			stdin,
			result: None,
			use_base64: arg.use_base64,
			protocol_version: arg.protocol_version,
			plugin_id: arg.plugin_id.to_string(),
		};

		let handle = HookHandle::executable(
			handle_inner,
			arg.plugin_id.to_string(),
			start_time,
			arg.persistence,
		);

		Ok(handle)
	}
}

/// Hook handler internals for an executable hook
pub(super) struct ExecutableHookHandle<H: Hook> {
	pub child: Child,
	pub stdout: TryLineReader<ChildStdout>,
	pub stdin: ChildStdin,
	pub result: Option<H::Result>,
	pub use_base64: bool,
	pub protocol_version: u16,
	pub plugin_id: String,
}

impl<H: Hook> ExecutableHookHandle<H> {
	/// Polls this hook, returning true if the polling is done and a result is available
	pub async fn poll(
		&mut self,
		plugin_persistence: &mut Option<Arc<Mutex<PluginPersistence>>>,
		command_results: &mut VecDeque<CommandResult>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<bool> {
		let lines = self.stdout.lines().await?;
		// EoF
		let Some(lines) = lines else {
			return Ok(true);
		};

		let persistence = plugin_persistence
			.as_mut()
			.context("Hook handle does not have a reference to persistent plugin data")?;
		let mut persistence_lock = persistence.lock().await;

		// Send command results from the worker to this hook
		if let Some(worker) = &mut persistence_lock.worker {
			while let Some(result) = worker.pop_command_result() {
				let action = InputAction::CommandResult(result)
					.serialize(self.protocol_version)
					.context("Failed to serialize input action")?;
				self.stdin
					.write(action.as_bytes())
					.await
					.context("Failed to write input action to plugin")?;
			}
		}

		for line in lines {
			let action = OutputAction::deserialize(&line, self.use_base64, self.protocol_version)
				.context("Failed to deserialize plugin action")?;

			let Some(action) = action else {
				if let Some(message) = line.strip_prefix("$_") {
					println!("{message}");
				}
				continue;
			};

			match action {
				OutputAction::SetResult(new_result) => {
					// Before version 3, this was just a string
					let new_result = if self.protocol_version < 3 {
						let string: String = serde_json::from_value(new_result)
							.context("Failed to deserialize hook result")?;
						serde_json::from_str(&string)
							.context("Failed to deserialize hook result")?
					} else {
						serde_json::from_value(new_result)
							.context("Failed to deserialize hook result")?
					};
					self.result = Some(new_result);

					// We can stop polling early
					return Ok(true);
				}
				OutputAction::SetError(error) => {
					return Err(anyhow!(
						"Plugin '{}' returned an error: {error}",
						self.plugin_id
					));
				}
				OutputAction::SetState(new_state) => {
					persistence_lock.state = new_state;
				}
				OutputAction::RunWorkerCommand { command, payload } => {
					let worker = persistence_lock.worker.as_mut().context(
						"Command was called on plugin worker, but the worker was not started",
					)?;
					worker
						.send_input_action(InputAction::Command { command, payload })
						.await
						.context("Failed to send command to worker")?;
				}
				OutputAction::SetCommandResult(result) => {
					command_results.push_back(result);
				}
				OutputAction::Text(text, level) => {
					o.display_text(text, level);
				}
				OutputAction::Message(message) => {
					o.display_message(message);
				}
				OutputAction::StartProcess => {
					o.start_process();
				}
				OutputAction::EndProcess => {
					o.end_process();
				}
				OutputAction::StartSection => {
					o.start_section();
				}
				OutputAction::EndSection => {
					o.end_section();
				}
			};
		}

		Ok(false)
	}

	pub async fn kill(mut self) -> anyhow::Result<Option<H::Result>> {
		self.child.kill().await?;

		Ok(self.result)
	}

	/// Gets the result from this hook. self.poll() must have already returned true for this to not throw an error.
	pub async fn result(mut self) -> anyhow::Result<H::Result> {
		let cmd_result = self.child.wait().await?;

		if !cmd_result.success() {
			if let Some(exit_code) = cmd_result.code() {
				bail!(
					"Hook from plugin '{}' returned a non-zero exit code of {}",
					self.plugin_id,
					exit_code
				);
			} else {
				bail!(
					"Hook from plugin '{}' returned a non-zero exit code",
					self.plugin_id
				);
			}
		}

		let result = self.result.with_context(|| {
			format!(
				"Plugin hook for plugin '{}' did not return a result",
				self.plugin_id
			)
		})?;

		Ok(result)
	}
}
