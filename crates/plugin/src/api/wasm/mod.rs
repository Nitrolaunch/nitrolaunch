/// ABI for host functions that the plugin can call
mod abi;
/// Filesystem access
pub mod fs;
/// System access
pub mod sys;
/// General utilities for the API
pub mod util;

use anyhow::{bail, Context};
use serde::de::DeserializeOwned;

use crate::hook::Hook;

/// Static where the hook result data is placed
pub static mut HOOK_RESULT: String = String::new();

/// Generates the exported functions for the entrypoint to your plugin
#[macro_export]
macro_rules! nitro_wasm_plugin {
	($func:ident, $id: literal) => {
		/// Returns a 0 on success, and a 1 on error
		#[no_mangle]
		pub extern "C" fn nitro_run_plugin(
			hook: *const u8,
			hook_len: usize,
			arg: *const u8,
			arg_len: usize,
			hook_version: u32,
		) -> u32 {
			let hook = unsafe { $crate::api::wasm::util::read_wasm_string(hook, hook_len) };
			let arg = unsafe { $crate::api::wasm::util::read_wasm_string(arg, arg_len) };

			let mut plugin = $crate::api::wasm::WASMPlugin {
				id: $id.to_string(),
				hook,
				arg,
				hook_version,
			};

			let result = $func(&mut plugin);
			let result_code = if let Err(e) = result {
				unsafe {
					$crate::api::wasm::set_hook_result(e.to_string());
				}
				1
			} else {
				0
			};

			result_code
		}
	};
}

/// A custom WASM plugin definition
pub struct WASMPlugin {
	/// The ID of the plugin
	pub id: String,
	/// The ID of the hook that is being run
	pub hook: &'static str,
	/// The argument to the hook that is being run
	pub arg: &'static str,
	/// The version of the hook that is being run
	pub hook_version: u32,
}

impl WASMPlugin {
	/// Handle a hook
	pub(crate) fn handle_hook<H: Hook>(
		&mut self,
		arg: impl FnOnce(&Self) -> anyhow::Result<H::Arg>,
		f: impl FnOnce(H::Arg) -> anyhow::Result<H::Result>,
	) -> anyhow::Result<()> {
		// Check if we are running the given hook
		if self.hook != H::get_name_static() {
			return Ok(());
		}

		// Check that the hook version of Nitrolaunch matches our hook version
		if self.hook_version != H::get_version() as u32 {
			bail!("Hook version does not match. Try updating the plugin or Nitrolaunch.");
		}

		let arg = arg(self)?;

		let result = f(arg);
		let result = match result {
			Ok(result) => result,
			Err(e) => {
				if H::get_takes_over() {
					eprintln!("Error in hook: {e:?}");
					return Ok(());
				} else {
					return Err(e);
				}
			}
		};

		if !H::get_takes_over() {
			// Output result last as it will make the plugin runner stop listening
			let serialized = serde_json::to_string(&result)?;
			unsafe { set_hook_result(serialized) };
		}

		Ok(())
	}

	/// Deserialize the main plugin arg as the hook input
	pub(crate) fn get_hook_arg<Arg: DeserializeOwned>(&self) -> anyhow::Result<Arg> {
		serde_json::from_str(self.arg).context("Failed to deserialize hook argument")
	}
}

/// Sets the result / error of the plugin hook
pub unsafe fn set_hook_result(result: String) {
	HOOK_RESULT = result;
}
