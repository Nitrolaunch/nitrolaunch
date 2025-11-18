use std::{marker::PhantomData, sync::Arc};

use anyhow::{bail, Context};
use nitro_shared::output::NitroOutput;
use wasmer::{imports, Module, Store, TypedFunction};

use crate::hook::{
	call::{HookCallArg, HookHandle},
	Hook, WASM_HOOK_ENTRYPOINT,
};

/// Calls a WASM hook handler
pub(crate) async fn call_wasm<H: Hook + Sized>(
	hook: &H,
	arg: HookCallArg<'_, H>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<HookHandle<H>> {
	let _ = hook;
	let _ = o;

	let data = tokio::fs::read(arg.cmd)
		.await
		.context("Failed to read WASM file")?;

	Ok(HookHandle::wasm(
		WASMHookHandle {
			plugin_id: arg.plugin_id.to_string(),
			code: data.into(),
			arg: serde_json::to_string(&arg.arg)?,
			result: None,
			_phantom: PhantomData,
		},
		arg.plugin_id.to_string(),
		arg.persistence,
	))
}

/// Hook handler internals for a WASM hook
pub(super) struct WASMHookHandle<H: Hook> {
	pub plugin_id: String,
	code: Arc<[u8]>,
	arg: String,
	result: Option<H::Result>,
	_phantom: PhantomData<H>,
}

impl<H: Hook> WASMHookHandle<H> {
	/// Runs this hook to completion
	pub async fn run(&mut self) -> anyhow::Result<()> {
		if self.result.is_some() {
			return Ok(());
		}

		// Initialize the module
		let mut store = Store::default();
		let module = Module::new(&store, &self.code).context("Failed to compile WASM module")?;

		let imports = imports! {};
		let instance = wasmer::Instance::new(&mut store, &module, &imports)
			.context("Failed to construct WASM instance")?;

		// Grab functions from the module
		let entrypoint: TypedFunction<(u32, u32, u32, u32, u16), u16> = instance
			.exports
			.get_typed_function(&store, WASM_HOOK_ENTRYPOINT)
			.context("WASM module does not export an entrypoint")?;

		let alloc_fn: TypedFunction<u32, u32> = instance
			.exports
			.get_typed_function(&store, "nitro_plugin_alloc")
			.context("WASM module is missing alloc function")?;

		let dealloc_fn: TypedFunction<(u32, u32), ()> = instance
			.exports
			.get_typed_function(&store, "nitro_plugin_dealloc")
			.context("WASM module is missing dealloc function")?;

		let get_result_fn: TypedFunction<(), u32> = instance
			.exports
			.get_typed_function(&store, "nitro_plugin_get_hook_result")
			.context("WASM module is missing get_result function")?;

		let get_result_len_fn: TypedFunction<(), u32> = instance
			.exports
			.get_typed_function(&store, "nitro_plugin_get_hook_result_len")
			.context("WASM module is missing get_result_len function")?;

		let memory = instance
			.exports
			.get_memory("memory")
			.context("WASM memory missing!")?;

		// Prepare the inputs by allocating strings in the module
		let hook_len = H::get_name_static().len() as u32;
		let hook_ptr_offset = alloc_fn.call(&mut store, hook_len)?;

		let view = memory.view(&store);
		view.write(hook_ptr_offset as u64, H::get_name_static().as_bytes())?;

		let arg_len = self.arg.len() as u32;
		let arg_ptr_offset = alloc_fn.call(&mut store, arg_len)?;

		let view = memory.view(&store);
		view.write(arg_ptr_offset as u64, self.arg.as_bytes())?;

		let result_code = entrypoint
			.call(
				&mut store,
				hook_ptr_offset,
				hook_len,
				arg_ptr_offset,
				arg_len,
				H::get_version(),
			)
			.context("Failed to call plugin entrypoint")?;

		// Deallocate inputs
		let _ = dealloc_fn.call(&mut store, hook_ptr_offset, hook_len);
		let _ = dealloc_fn.call(&mut store, arg_ptr_offset, arg_len);

		// Read the result from the memory
		let result_ptr_offset = get_result_fn.call(&mut store)?;
		let result_len = get_result_len_fn.call(&mut store)?;

		let view = memory.view(&store);
		let mut result: String = std::iter::repeat_n('0', result_len as usize).collect();
		unsafe { view.read(result_ptr_offset as u64, result.as_bytes_mut())? };

		if result_code == 1 {
			bail!("Plugin '{}' returned an error: {result}", self.plugin_id);
		}

		let result = unsafe { simd_json::from_str(&mut result) }
			.context("Failed to deserialize hook result")?;

		self.result = Some(result);

		Ok(())
	}

	/// Gets the hook result if the hook has been run
	pub fn result(self) -> Option<H::Result> {
		self.result
	}
}
