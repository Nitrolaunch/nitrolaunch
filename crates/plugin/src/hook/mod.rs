use nitro_shared::output::NitroOutput;
use serde::{de::DeserializeOwned, Serialize};

use crate::hook::call::{HookCallArg, HookHandle};

/// Implementation for calling hooks
pub mod call;
/// Calling hooks with executables
pub mod executable;
/// Hook definitions
pub mod hooks;
/// WASM hook execution
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
