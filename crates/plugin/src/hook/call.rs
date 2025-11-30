use std::{
	collections::VecDeque,
	path::Path,
	sync::Arc,
	time::{Duration, Instant},
};

use crate::{
	hook::{
		executable::ExecutableHookHandle,
		wasm::{loader::WASMLoader, WASMHookHandle},
		Hook,
	},
	PluginPaths,
};
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput, NoOp};
use tokio::sync::Mutex;

use crate::{
	input_output::{CommandResult, InputAction},
	plugin::PluginPersistence,
};

/// Argument struct for the hook call function
pub struct HookCallArg<'a, H: Hook> {
	/// The command or WASM file to run
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
	pub paths: &'a PluginPaths,
	/// The version of Nitrolaunch
	pub nitro_version: Option<&'a str>,
	/// The ID of the plugin
	pub plugin_id: &'a str,
	/// The list of all enabled plugins and their versions
	pub plugin_list: &'a [String],
	/// The protocol version
	pub protocol_version: u16,
	/// The WASM file loader
	pub wasm_loader: Arc<Mutex<WASMLoader>>,
}

/// Handle returned by running a hook. Make sure to await it if you need to.
#[must_use]
pub struct HookHandle<H: Hook> {
	inner: HookHandleInner<H>,
	plugin_persistence: Option<Arc<Mutex<PluginPersistence>>>,
	plugin_id: String,
	command_results: VecDeque<CommandResult>,
	start_time: Option<Instant>,
	/// Whether poll() has returned true
	is_finished: bool,
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
			is_finished: true,
		}
	}

	/// Create a new executable handle
	pub(super) fn executable(
		inner: ExecutableHookHandle<H>,
		plugin_id: String,
		plugin_persistence: Arc<Mutex<PluginPersistence>>,
	) -> Self {
		Self {
			inner: HookHandleInner::Executable(inner),
			plugin_persistence: Some(plugin_persistence),
			plugin_id,
			command_results: VecDeque::new(),
			start_time: None,
			is_finished: false,
		}
	}

	/// Create a new WASM handle
	pub(super) fn wasm(
		inner: WASMHookHandle<H>,
		plugin_id: String,
		plugin_persistence: Arc<Mutex<PluginPersistence>>,
	) -> Self {
		let start_time = if std::env::var("NITRO_PLUGIN_PROFILE").is_ok_and(|x| x == "1") {
			Some(Instant::now())
		} else {
			None
		};

		Self {
			inner: HookHandleInner::WASM(inner),
			plugin_persistence: Some(plugin_persistence),
			plugin_id,
			command_results: VecDeque::new(),
			start_time,
			is_finished: false,
		}
	}

	/// Get the ID of the plugin that returned this handle
	pub fn get_id(&self) -> &String {
		&self.plugin_id
	}

	/// Ensures that this hook has started
	pub async fn ensure_started(&mut self, o: &mut impl NitroOutput) -> anyhow::Result<()> {
		if let HookHandleInner::Executable(inner) = &mut self.inner {
			inner
				.ensure_started(
					&mut self.plugin_persistence,
					&mut self.command_results,
					&mut self.start_time,
					o,
				)
				.await?;
		}

		Ok(())
	}

	/// Poll the handle, returning true if the handle is ready
	pub async fn poll(&mut self, o: &mut impl NitroOutput) -> anyhow::Result<bool> {
		if self.is_finished {
			return Ok(true);
		}

		let finished = match &mut self.inner {
			HookHandleInner::Executable(inner) => {
				inner
					.poll(
						&mut self.plugin_persistence,
						&mut self.command_results,
						&mut self.start_time,
						o,
					)
					.await?
			}
			HookHandleInner::WASM(inner) => {
				inner.run(o).await?;

				true
			}
			HookHandleInner::Constant(..) => true,
		};

		if finished {
			self.is_finished = true;

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
			inner.send_input_action(action).await?;
		}

		Ok(())
	}

	/// Get the result of the hook by waiting for it
	pub async fn result(mut self, o: &mut impl NitroOutput) -> anyhow::Result<H::Result> {
		if let HookHandleInner::Executable(..) | HookHandleInner::WASM(..) = &self.inner {
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
			HookHandleInner::WASM(inner) => Ok(inner.result().expect("Hook has not been polled")),
		}
	}

	/// Get the result of the hook by killing it
	pub async fn kill(self, o: &mut impl NitroOutput) -> anyhow::Result<Option<H::Result>> {
		let _ = o;
		match self.inner {
			HookHandleInner::Constant(result) => Ok(Some(result)),
			HookHandleInner::Executable(inner) => inner.kill().await,
			HookHandleInner::WASM(inner) => Ok(inner.result()),
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
	/// Result is coming from WASM code
	WASM(WASMHookHandle<H>),
	/// Result is a constant, either from a constant hook or a takeover hook
	Constant(H::Result),
}

/// A collection of HookHandles that can be run. Ensures that proper ordering of results is upheld.
pub struct HookHandles<H: Hook> {
	handles: VecDeque<HookHandle<H>>,
}

impl<H: Hook> HookHandles<H> {
	pub(crate) async fn new(
		mut handles: VecDeque<HookHandle<H>>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Self> {
		// Asynchronous hooks can all be started so that they run at the same time
		if H::is_asynchronous() {
			for handle in &mut handles {
				handle.ensure_started(o).await?;
			}
		}

		Ok(Self { handles })
	}

	/// Gets whether there are any handles in the queue
	pub fn is_empty(&self) -> bool {
		self.handles.is_empty()
	}

	/// Gets the number of handles still in the queue
	pub fn len(&self) -> usize {
		self.handles.len()
	}

	/// Gets the next handle in the queue
	pub fn next(&mut self) -> Option<HookHandle<H>> {
		self.handles.pop_front()
	}

	/// Gets the result from the next handle in the queue, returning None if empty
	pub async fn next_result(
		&mut self,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<H::Result>> {
		let Some(next) = self.next() else {
			return Ok(None);
		};

		next.result(o).await.map(Some)
	}

	/// Gets the results from all handles, storing them in a vec
	pub async fn all_results(mut self, o: &mut impl NitroOutput) -> anyhow::Result<Vec<H::Result>> {
		let mut out = Vec::with_capacity(self.len());
		while let Some(result) = self.next_result(o).await? {
			out.push(result);
		}

		Ok(out)
	}

	/// Polls all the hooks in the queue.
	/// Note that this is only valid behavior for certain hooks that are long-running such as WhileInstanceLaunch.
	pub async fn poll_all(&mut self, o: &mut impl NitroOutput) -> anyhow::Result<()> {
		for handle in &mut self.handles {
			handle.poll(o).await?;
		}

		Ok(())
	}

	/// Terminates all the handles in the queue
	pub async fn terminate(self) {
		for handle in self.handles {
			handle.terminate().await;
		}
	}

	/// Kills all the handles in the queue
	pub async fn kill(self, o: &mut impl NitroOutput) -> anyhow::Result<Vec<H::Result>> {
		// We store the error throughout to ensure that all of the handles are still killed
		let mut error = None;
		let mut out = Vec::new();
		for handle in self.handles {
			match handle.kill(o).await {
				Ok(result) => out.extend(result),
				Err(e) => error = Some(e),
			}
		}

		if let Some(error) = error {
			Err(error)
		} else {
			Ok(out)
		}
	}
}

impl<H: Hook, T> HookHandles<H>
where
	H::Result: IntoIterator<Item = T>,
{
	/// Gets the results from all handles, flattening them from lists and storing them in a vec
	pub async fn flatten_all_results(mut self, o: &mut impl NitroOutput) -> anyhow::Result<Vec<T>> {
		let mut out = Vec::new();
		while let Some(result) = self.next_result(o).await? {
			out.extend(result);
		}

		Ok(out)
	}
}

impl<H: Hook, T> HookHandles<H>
where
	H::Result: OptionLike<Type = T>,
{
	/// Gets the results from handles, returning the first non-None value
	pub async fn first_some(mut self, o: &mut impl NitroOutput) -> anyhow::Result<Option<T>> {
		while let Some(result) = self.next_result(o).await? {
			if let Some(result) = result.into_option() {
				return Ok(Some(result));
			}
		}

		Ok(None)
	}
}

/// Utitlity trait for Option<T>
pub trait OptionLike {
	/// T
	type Type;
	/// Converts to an Option<T>
	fn into_option(self) -> Option<Self::Type>;
}

impl<T> OptionLike for Option<T> {
	type Type = T;
	fn into_option(self) -> Option<Self::Type> {
		self
	}
}
