use std::{
	collections::VecDeque,
	path::Path,
	sync::Arc,
	time::{Duration, Instant},
};

use crate::hook::{executable::ExecutableHookHandle, Hook};
use anyhow::Context;
use nitro_core::Paths;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput, NoOp};
use tokio::{io::AsyncWriteExt, sync::Mutex};

use crate::{
	input_output::{CommandResult, InputAction},
	plugin::PluginPersistence,
};

/// Argument struct for the hook call function
pub struct HookCallArg<'a, H: Hook> {
	/// The command to run
	pub cmd: &'a str,
	/// The argument to the hook
	pub arg: &'a H::Arg,
	/// Additional arguments for the executable
	pub additional_args: &'a [String],
	/// The working directory for the executable
	pub working_dir: Option<&'a Path>,
	/// Whether to use base64 encoding
	pub use_base64: bool,
	/// Custom configuration for the plugin
	pub custom_config: Option<String>,
	/// Persistent data for the plugin
	pub persistence: Arc<Mutex<PluginPersistence>>,
	/// Paths
	pub paths: &'a Paths,
	/// The version of Nitrolaunch
	pub nitro_version: Option<&'a str>,
	/// The ID of the plugin
	pub plugin_id: &'a str,
	/// The list of all enabled plugins and their versions
	pub plugin_list: &'a [String],
	/// The protocol version
	pub protocol_version: u16,
}

/// Handle returned by running a hook. Make sure to await it if you need to.
#[must_use]
pub struct HookHandle<H: Hook> {
	inner: HookHandleInner<H>,
	plugin_persistence: Option<Arc<Mutex<PluginPersistence>>>,
	plugin_id: String,
	command_results: VecDeque<CommandResult>,
	start_time: Option<Instant>,
}

impl<H: Hook> HookHandle<H> {
	/// Create a new constant handle
	pub fn constant(result: H::Result, plugin_id: String) -> Self {
		Self {
			inner: HookHandleInner::Constant(result),
			plugin_persistence: None,
			plugin_id,
			command_results: VecDeque::new(),
			start_time: None,
		}
	}

	/// Create a new executable handle
	pub(super) fn executable(
		inner: ExecutableHookHandle<H>,
		plugin_id: String,
		start_time: Option<Instant>,
	) -> Self {
		Self {
			inner: HookHandleInner::Executable(inner),
			plugin_persistence: None,
			plugin_id,
			command_results: VecDeque::new(),
			start_time,
		}
	}

	/// Get the ID of the plugin that returned this handle
	pub fn get_id(&self) -> &String {
		&self.plugin_id
	}

	/// Poll the handle, returning true if the handle is ready
	pub async fn poll(&mut self, o: &mut impl NitroOutput) -> anyhow::Result<bool> {
		let finished = match &mut self.inner {
			HookHandleInner::Executable(inner) => {
				inner
					.poll(&mut self.plugin_persistence, &mut self.command_results, o)
					.await?
			}
			HookHandleInner::Constant(..) => true,
		};

		if finished {
			if let Some(start_time) = &self.start_time {
				let now = Instant::now();
				let delta = now.duration_since(*start_time);
				o.display(
					MessageContents::Simple(format!(
						"Plugin '{}' took {delta:?} to run hook '{}'",
						self.plugin_id,
						H::get_name_static()
					)),
					MessageLevel::Important,
				);
			}
		}

		Ok(finished)
	}

	/// Sends an action to the plugin
	pub async fn send_input_action(&mut self, action: InputAction) -> anyhow::Result<()> {
		if let HookHandleInner::Executable(inner) = &mut self.inner {
			let action = action
				.serialize(inner.protocol_version)
				.context("Failed to serialize input action")?;

			inner
				.stdin
				.write(action.as_bytes())
				.await
				.context("Failed to write input action to plugin")?;
		}

		Ok(())
	}

	/// Get the result of the hook by waiting for it
	pub async fn result(mut self, o: &mut impl NitroOutput) -> anyhow::Result<H::Result> {
		if let HookHandleInner::Executable(..) = &self.inner {
			loop {
				let result = self.poll(o).await?;
				if result {
					break;
				}
				tokio::time::sleep(Duration::from_micros(50)).await;
			}
		}

		match self.inner {
			HookHandleInner::Constant(result) => Ok(result),
			HookHandleInner::Executable(inner) => inner.result().await,
		}
	}

	/// Get the result of the hook by killing it
	pub async fn kill(self, o: &mut impl NitroOutput) -> anyhow::Result<Option<H::Result>> {
		let _ = o;
		match self.inner {
			HookHandleInner::Constant(result) => Ok(Some(result)),
			HookHandleInner::Executable(inner) => inner.kill().await,
		}
	}

	/// Terminate the hook gracefully, without getting the result
	pub async fn terminate(mut self) {
		let result = self.send_input_action(InputAction::Terminate).await;
		if result.is_err() {
			let _ = self.kill(&mut NoOp).await;
		}
	}

	/// Pops a command result from this hook handle
	pub fn pop_command_result(&mut self) -> Option<CommandResult> {
		self.command_results.pop_front()
	}
}

/// The inner value for a HookHandle
enum HookHandleInner<H: Hook> {
	/// Result is coming from an executable
	Executable(ExecutableHookHandle<H>),
	/// Result is a constant, either from a constant hook or a takeover hook
	Constant(H::Result),
}
