use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	time::UNIX_EPOCH,
};

use anyhow::{bail, Context};
use wasmtime::{component::Component, Config, Engine};

/// Manager for loading and caching WASM efficiently
pub struct WASMLoader {
	/// Directory where WASM will be cached
	cache_dir: PathBuf,
	/// Map of plugin IDs to already loaded components
	component_cache: HashMap<String, Component>,
	/// The engine used for this loader
	engine: Engine,
}

impl WASMLoader {
	/// Creates a new WASMLoader with the default cache directory
	pub fn new(data_dir: &Path) -> Self {
		Self::with_cache_dir(data_dir.join("internal/wasm_cache"))
	}

	/// Creates a new WASMLoader with the given cache directory path
	pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
		let engine =
			Engine::new(Config::new().async_support(true)).expect("Failed to create engine");
		Self {
			cache_dir,
			component_cache: HashMap::new(),
			engine,
		}
	}

	/// Loads the WASM for a plugin
	pub async fn load(&mut self, plugin_id: String, wasm_file: &Path) -> anyhow::Result<Component> {
		if let Some(component) = self.component_cache.get(&plugin_id) {
			return Ok(component.clone());
		}

		if !wasm_file.exists() {
			bail!("Plugin WASM file does not exist");
		}

		if !self.cache_dir.exists() {
			let _ = tokio::fs::create_dir_all(&self.cache_dir).await;
		}

		// Check the WASM last modified timestamp with a stored one to see if we need to recompile
		let cached_file_path = self.cache_dir.join(format!("{plugin_id}.wasmtime"));
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

		// Recompile the component if needed, otherwise read the assembly from the file
		let component = if recomp_needed {
			let contents = tokio::fs::read(wasm_file)
				.await
				.context("Failed to read WASM file")?;
			let engine = self.engine.clone();
			let component = tokio::task::spawn_blocking(move || Component::new(&engine, contents))
				.await
				.context("Failed to compile WASM file")??;

			if let Ok(bytes) = component.serialize() {
				// Don't block the load on writing the compiled file
				tokio::spawn(async move {
					if tokio::fs::write(cached_file_path, bytes).await.is_ok() {
						if let Some(last_modified) = last_modified {
							let _ =
								tokio::fs::write(timestamp_path, last_modified.to_string()).await;
						}
					}
				});
			}

			component
		} else {
			// SAFETY: None really
			let component = unsafe { Component::deserialize_file(&self.engine, &cached_file_path) };

			// Recompile the component if it is malformed
			if let Ok(component) = component {
				component
			} else {
				if cached_file_path.exists() {
					std::fs::remove_file(&cached_file_path)?;
				}
				return Box::pin(self.load(plugin_id, wasm_file)).await;
			}
		};

		self.component_cache.insert(plugin_id, component.clone());

		Ok(component)
	}

	/// Gets this loader's engine
	pub fn engine(&self) -> Engine {
		self.engine.clone()
	}
}
