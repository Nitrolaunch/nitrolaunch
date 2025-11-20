/// Manager for loading and caching WASM efficiently
pub mod loader;

use std::{marker::PhantomData, path::PathBuf, sync::Arc, time::Instant};

use anyhow::{bail, Context};
use nitro_shared::{later::Later, output::NitroOutput};
use tokio::sync::Mutex;
use wasmtime::{Caller, Linker, Memory, Store, TypedFunc};

use crate::hook::{
	call::{HookCallArg, HookHandle},
	wasm::loader::WASMLoader,
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

	Ok(HookHandle::wasm(
		WASMHookHandle {
			plugin_id: arg.plugin_id.to_string(),
			wasm_path: PathBuf::from(arg.cmd),
			arg: serde_json::to_string(&arg.arg)?,
			result: None,
			custom_config: arg.custom_config,
			wasm_loader: arg.wasm_loader,
			_phantom: PhantomData,
		},
		arg.plugin_id.to_string(),
		arg.persistence,
	))
}

/// Hook handler internals for a WASM hook
pub(super) struct WASMHookHandle<H: Hook> {
	pub plugin_id: String,
	wasm_path: PathBuf,
	arg: String,
	result: Option<H::Result>,
	custom_config: Option<String>,
	wasm_loader: Arc<Mutex<WASMLoader>>,
	_phantom: PhantomData<H>,
}

impl<H: Hook> WASMHookHandle<H> {
	/// Runs this hook to completion
	pub async fn run(&mut self) -> anyhow::Result<()> {
		if self.result.is_some() {
			return Ok(());
		}

		let mut start_time = if std::env::var("NITRO_PLUGIN_PROFILE").is_ok_and(|x| x == "1") {
			Some(Instant::now())
		} else {
			None
		};

		// Initialize the module
		let mut lock = self.wasm_loader.lock().await;
		let module = lock
			.load(self.plugin_id.clone(), &self.wasm_path)
			.await
			.context("Failed to load WASM module")?;
		let engine = lock.engine();
		std::mem::drop(lock);

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Module initialization: {:?}", now - *start_time);
			*start_time = now;
		}

		let mut linker = Linker::new(&engine);

		let mut store = Store::new(
			&engine,
			State {
				memory: Later::new(),
				alloc_fn: Later::new(),
				custom_config: self.custom_config.clone(),
			},
		);

		create_imports(&mut linker).context("Failed to create imports")?;

		let instance = linker
			.instantiate(&mut store, &module)
			.context("Failed to construct WASM instance")?;

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Instance initialization: {:?}", now - *start_time);
			*start_time = now;
		}

		// Grab functions from the module
		let entrypoint: TypedFunc<(u32, u32, u32, u32, u32), u32> = instance
			.get_typed_func(&mut store, WASM_HOOK_ENTRYPOINT)
			.context("WASM module does not export an entrypoint")?;

		let alloc_fn: TypedFunc<u32, u32> = instance
			.get_typed_func(&mut store, "nitro_plugin_alloc")
			.context("WASM module is missing alloc function")?;

		let dealloc_fn: TypedFunc<(u32, u32), ()> = instance
			.get_typed_func(&mut store, "nitro_plugin_dealloc")
			.context("WASM module is missing dealloc function")?;

		let get_result_fn: TypedFunc<(), u32> = instance
			.get_typed_func(&mut store, "nitro_plugin_get_hook_result")
			.context("WASM module is missing get_result function")?;

		let get_result_len_fn: TypedFunc<(), u32> = instance
			.get_typed_func(&mut store, "nitro_plugin_get_hook_result_len")
			.context("WASM module is missing get_result_len function")?;

		let memory = instance
			.get_memory(&mut store, "memory")
			.context("WASM memory missing!")?;

		// Prepare the env
		store.data_mut().memory.fill(memory.clone());
		store.data_mut().alloc_fn.fill(alloc_fn.clone());

		// Prepare the inputs by allocating strings in the module
		let hook_len = H::get_name_static().len() as u32;
		let hook_ptr_offset = alloc_fn.call(&mut store, hook_len)?;

		memory.write(
			&mut store,
			hook_ptr_offset as usize,
			H::get_name_static().as_bytes(),
		)?;

		let arg_len = self.arg.len() as u32;
		let arg_ptr_offset = alloc_fn.call(&mut store, arg_len)?;

		memory.write(&mut store, arg_ptr_offset as usize, self.arg.as_bytes())?;

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Function and input setup: {:?}", now - *start_time);
			*start_time = now;
		}

		let result_code = entrypoint
			.call(
				&mut store,
				(
					hook_ptr_offset,
					hook_len,
					arg_ptr_offset,
					arg_len,
					H::get_version() as u32,
				),
			)
			.context("Failed to call plugin entrypoint")?;

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Hook runtime: {:?}", now - *start_time);
			*start_time = now;
		}

		// Deallocate inputs
		let _ = dealloc_fn.call(&mut store, (hook_ptr_offset, hook_len));
		let _ = dealloc_fn.call(&mut store, (arg_ptr_offset, arg_len));

		// Read the result from the memory
		let result_ptr_offset = get_result_fn.call(&mut store, ())?;
		let result_len = get_result_len_fn.call(&mut store, ())?;

		let mut result: String = std::iter::repeat_n('0', result_len as usize).collect();
		unsafe { memory.read(&store, result_ptr_offset as usize, result.as_bytes_mut())? };

		if result_code == 1 {
			bail!("Plugin '{}' returned an error: {result}", self.plugin_id);
		}

		let result = unsafe { simd_json::from_str(&mut result) }
			.context("Failed to deserialize hook result")?;

		self.result = Some(result);

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Result handling: {:?}", now - *start_time);
			*start_time = now;
		}

		Ok(())
	}

	/// Gets the hook result if the hook has been run
	pub fn result(self) -> Option<H::Result> {
		self.result
	}
}

/// Host function environment
struct State {
	memory: Later<Memory>,
	alloc_fn: Later<TypedFunc<u32, u32>>,
	custom_config: Option<String>,
}

fn create_imports(linker: &mut Linker<State>) -> anyhow::Result<()> {
	let get_custom_config = move |mut caller: Caller<State>| {
		let state = caller.data_mut();
		let custom_config = state.custom_config.clone();
		let memory = state.memory.get_clone();
		let alloc_fn = state.alloc_fn.get_clone();

		if let Some(custom_config) = &custom_config {
			let Ok(ptr) = alloc_fn.call(&mut caller, custom_config.len() as u32) else {
				return (0, 0);
			};

			if memory
				.write(&mut caller, ptr as usize, custom_config.as_bytes())
				.is_err()
			{
				return (0, 0);
			}

			(ptr as u64, custom_config.len() as u64)
		} else {
			(0, 0)
		}
	};

	linker.func_wrap("nitro", "get_custom_config", get_custom_config)?;

	Ok(())
}
