use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	time::UNIX_EPOCH,
};

use anyhow::{bail, Context};
use wasmer::{Module, Store};

/// Manager for loading and caching WASM efficiently
pub struct WASMLoader {
	/// Directory where WASM will be cached
	cache_dir: PathBuf,
	/// Map of plugin IDs to already loaded modules
	module_cache: HashMap<String, Module>,
}

impl WASMLoader {
	/// Creates a new WASMLoader with the given cache directory path
	pub fn new(cache_dir: PathBuf) -> Self {
		Self {
			cache_dir,
			module_cache: HashMap::new(),
		}
	}

	/// Loads the WASM for a plugin
	pub async fn load(&mut self, plugin_id: String, wasm_file: &Path) -> anyhow::Result<Module> {
		if let Some(module) = self.module_cache.get(&plugin_id) {
			return Ok(module.clone());
		}

		if !wasm_file.exists() {
			bail!("Plugin WASM file does not exist");
		}

		if !self.cache_dir.exists() {
			let _ = tokio::fs::create_dir_all(&self.cache_dir).await;
		}

		// Check the WASM last modified timestamp with a stored one to see if we need to recompile
		let cached_file_path = self.cache_dir.join(format!("{plugin_id}.wasmr"));
		let timestamp_path = self.cache_dir.join(format!("{plugin_id}.timestamp"));

		let metadata = tokio::fs::metadata(wasm_file).await?;
		let last_modified = metadata
			.modified()
			.ok()
			.and_then(|x| x.duration_since(UNIX_EPOCH).ok())
			.map(|x| x.as_secs());

		let recomp_needed = if !timestamp_path.exists() || !cached_file_path.exists() {
			true
		} else {
			let expected_timestamp = tokio::fs::read_to_string(&timestamp_path).await;

			if let (Ok(expected_timestamp), Some(modified)) = (expected_timestamp, last_modified) {
				if let Ok(expected_timestamp) = expected_timestamp.trim().parse::<u64>() {
					expected_timestamp != modified
				} else {
					true
				}
			} else {
				true
			}
		};

		let store = Store::default();

		// Recompile the module if needed, otherwise read the assembly from the file
		let module = if recomp_needed {
			let contents = tokio::fs::read(wasm_file)
				.await
				.context("Failed to read WASM file")?;
			let module = tokio::task::spawn_blocking(move || {
				let store = Store::default();
				Module::new(&store, contents)
			})
			.await
			.context("Failed to compile WASM file")??;

			if module.serialize_to_file(&cached_file_path).is_ok() {
				if let Some(last_modified) = last_modified {
					tokio::fs::write(timestamp_path, last_modified.to_string()).await?;
				}
			}

			module
		} else {
			// SAFETY: None really
			unsafe {
				Module::deserialize_from_file(&store, &cached_file_path)
					.context("Failed to deserialize compiled file")?
			}
		};

		self.module_cache.insert(plugin_id, module.clone());

		Ok(module)
	}
}
