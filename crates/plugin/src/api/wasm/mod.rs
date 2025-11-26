#![allow(missing_docs)]

/// Interfacing code
pub mod interface;
/// System access
pub mod sys;
/// General utilities for the API
pub mod util;

use anyhow::{bail, Context};
use serde::de::DeserializeOwned;

use crate::hook::Hook;

pub use interface::export;
pub use interface::Guest;

/// Static where the hook result data is placed
static mut HOOK_RESULT: String = String::new();

/// Generates the exported functions for the entrypoint to your plugin
#[macro_export]
macro_rules! nitro_wasm_plugin {
	($func:ident, $id: literal) => {
		struct ExportedWASMPlugin;

		impl $crate::api::wasm::Guest for ExportedWASMPlugin {
			fn run_plugin(hook: String, arg: String, hook_version: u32) -> u32 {
				let mut plugin = $crate::api::wasm::WASMPlugin {
					id: $id.to_string(),
					hook: hook.to_string(),
					arg: arg.to_string(),
					hook_version,
				};

				let result = $func(&mut plugin);

				if let Err(e) = result {
					unsafe {
						$crate::api::wasm::_set_hook_result(e.to_string());
					}
					1
				} else {
					0
				}
			}

			fn get_result() -> String {
				unsafe { $crate::api::wasm::_get_hook_result() }
			}
		}

		$crate::api::wasm::export!(ExportedWASMPlugin with_types_in $crate::api::wasm::interface);
	};
}

/// A custom WASM plugin definition
pub struct WASMPlugin {
	/// The ID of the plugin
	pub id: String,
	/// The ID of the hook that is being run
	pub hook: String,
	/// The argument to the hook that is being run
	pub arg: String,
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
			unsafe { _set_hook_result(serialized) };
		}

		Ok(())
	}

	/// Deserialize the main plugin arg as the hook input
	pub(crate) fn get_hook_arg<Arg: DeserializeOwned>(&self) -> anyhow::Result<Arg> {
		serde_json::from_str(&self.arg).context("Failed to deserialize hook argument")
	}
}

/// Sets the result / error of the plugin hook
pub unsafe fn _set_hook_result(result: String) {
	HOOK_RESULT = result;
}

/// Get the result from the hook that was run. Internal function only used by the ABI.
pub unsafe fn _get_hook_result() -> String {
	#[allow(static_mut_refs)]
	HOOK_RESULT.clone()
}
