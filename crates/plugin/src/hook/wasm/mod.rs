/// Manager for loading and caching WASM efficiently
pub mod loader;

use std::{marker::PhantomData, path::PathBuf, sync::Arc, time::Instant};

use anyhow::{bail, Context};
use nitro_shared::{
	output::NitroOutput,
	util::{ARCH_STRING, OS_STRING},
};
use tokio::sync::Mutex;
use wasmtime::{
	component::{HasSelf, Linker},
	Store,
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::hook::{
	call::{HookCallArg, HookHandle},
	wasm::loader::WASMLoader,
	Hook,
};

#[allow(missing_docs)]
mod bindings {
	wasmtime::component::bindgen!({
		path: "src/interface.wit",
		imports: { default: async },
		exports: { default: async }
	});
}

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
			data_dir: arg.paths.data_dir.to_string_lossy().to_string(),
			config_dir: arg.paths.config_dir.to_string_lossy().to_string(),
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
	data_dir: String,
	config_dir: String,
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

		// Initialize the component
		let mut lock = self.wasm_loader.lock().await;
		let component = lock
			.load(self.plugin_id.clone(), &self.wasm_path)
			.await
			.context("Failed to load WASM component")?;
		let engine = lock.engine();
		std::mem::drop(lock);

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Component initialization: {:?}", now - *start_time);
			*start_time = now;
		}

		let mut linker = Linker::new(&engine);

		let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();
		wasmtime_wasi::p2::add_to_linker_async(&mut linker)
			.context("Failed to add WASI functions to linker")?;

		bindings::InterfaceWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |x| x)?;

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Linker initialization: {:?}", now - *start_time);
			*start_time = now;
		}

		let mut store = Store::new(
			&engine,
			State {
				wasi_ctx,
				table: ResourceTable::new(),
				custom_config: self.custom_config.clone(),
				data_dir: self.data_dir.clone(),
				config_dir: self.config_dir.clone(),
			},
		);

		let instance = bindings::InterfaceWorld::instantiate_async(&mut store, &component, &linker)
			.await
			.context("Failed to construct WASM instance")?;

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Instance initialization: {:?}", now - *start_time);
			*start_time = now;
		}

		let result_code = instance
			.call_run_plugin(
				&mut store,
				H::get_name_static(),
				&self.arg,
				H::get_version() as u32,
			)
			.await
			.context("Failed to call plugin entrypoint")?;

		if let Some(start_time) = &mut start_time {
			let now = Instant::now();
			println!("Hook runtime: {:?}", now - *start_time);
			*start_time = now;
		}

		let mut result = instance.call_get_result(&mut store).await?;

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
	wasi_ctx: WasiCtx,
	table: ResourceTable,
	custom_config: Option<String>,
	data_dir: String,
	config_dir: String,
}

impl WasiView for State {
	fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
		WasiCtxView {
			ctx: &mut self.wasi_ctx,
			table: &mut self.table,
		}
	}
}

impl bindings::InterfaceWorldImports for State {
	async fn get_custom_config(&mut self) -> Option<String> {
		self.custom_config.clone()
	}

	async fn get_data_dir(&mut self) -> String {
		self.data_dir.clone()
	}

	async fn get_config_dir(&mut self) -> String {
		self.config_dir.clone()
	}

	async fn get_os_string(&mut self) -> String {
		OS_STRING.to_string()
	}

	async fn get_arch_string(&mut self) -> String {
		ARCH_STRING.to_string()
	}

	async fn get_pointer_width(&mut self) -> u32 {
		#[cfg(target_pointer_width = "32")]
		return 32;
		#[cfg(target_pointer_width = "64")]
		return 64;
	}
}
