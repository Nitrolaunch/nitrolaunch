use anyhow::Context;
use nitrolaunch::config::Config;
use nitrolaunch::io::paths::Paths;
use nitrolaunch::plugin::PluginManager;
use nitrolaunch::plugin_crate::hook::wasm::loader::WASMLoader;
use nitrolaunch::shared::output::NitroOutput;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::State;

pub mod instance;
pub mod launch;
pub mod misc;
pub mod package;
pub mod plugin;
pub mod settings;
pub mod transfer;
pub mod user;

async fn load_config(
	paths: &Paths,
	wasm_loader: &Arc<Mutex<WASMLoader>>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<Config> {
	let plugins = PluginManager::load(paths, o)
		.await
		.context("Failed to load plugin manager")?;

	plugins.set_wasm_loader(wasm_loader.clone()).await;

	Config::load(
		&Config::get_path(paths),
		plugins,
		true,
		paths,
		crate::get_ms_client_id(),
		o,
	)
	.await
	.context("Failed to load config")
}

/// Error formatting for results
fn fmt_err<T, E: Debug>(r: Result<T, E>) -> Result<T, String> {
	r.map_err(|x| format!("{x:?}"))
}

/// Cancels a task
#[tauri::command]
pub async fn cancel_task(state: tauri::State<'_, State>, task: &str) -> Result<(), String> {
	state.task_manager.get().unwrap().lock().await.kill(task);

	Ok(())
}
