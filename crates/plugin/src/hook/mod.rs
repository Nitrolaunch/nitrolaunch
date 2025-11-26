#[cfg(feature = "host")]
use nitro_shared::output::NitroOutput;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "host")]
use crate::hook::call::{HookCallArg, HookHandle};

/// Implementation for calling hooks
#[cfg(feature = "host")]
pub mod call;
/// Calling hooks with executables
#[cfg(feature = "host")]
pub mod executable;
/// Hook definitions
pub mod hooks;
/// WASM hook execution
#[cfg(feature = "host")]
pub mod wasm;

/// Trait for a hook that can be called
pub trait Hook {
	/// The type for the argument that goes into the hook
	type Arg: Serialize + DeserializeOwned;
	/// The type for the result from the hook
	type Result: DeserializeOwned + Serialize + Default;

	/// Get the name of the hook
	fn get_name(&self) -> &'static str {
		Self::get_name_static()
	}

	/// Get the name of the hook statically
	fn get_name_static() -> &'static str;

	/// Get whether the hook should forward all output to the terminal
	fn get_takes_over() -> bool {
		false
	}

	/// Get whether the hook can be run asynchronously
	fn is_asynchronous() -> bool {
		false
	}

	/// Get the version number of the hook
	fn get_version() -> u16;

	/// Call the hook using the specified program
	#[cfg(feature = "host")]
	#[allow(async_fn_in_trait)]
	async fn call(
		&self,
		arg: HookCallArg<'_, Self>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<HookHandle<Self>>
	where
		Self: Sized,
	{
		crate::hook::executable::call_executable(self, arg, o).await
	}
}

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

/// Filename for a plugin's WASM code
pub static WASM_FILE_NAME: &str = "plugin.wasm";
