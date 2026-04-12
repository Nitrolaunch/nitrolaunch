use std::{borrow::Borrow, collections::HashMap, hash::Hash, marker::PhantomData, path::PathBuf};

use anyhow::{anyhow, Context};
use nitro_config::{instance::InstanceConfig, template::TemplateConfig};
use serde::de::DeserializeOwned;

/// Gets the map of available instances
pub fn get_instances() -> Option<WASMMap<InstanceConfig>> {
	let instances = super::interface::get_instances()?;

	Some(WASMMap {
		map: instances.into_iter().collect(),
		_phantom: PhantomData,
	})
}

/// Gets the map of available templates
pub fn get_templates() -> Option<WASMMap<TemplateConfig>> {
	let templates = super::interface::get_templates()?;

	Some(WASMMap {
		map: templates.into_iter().collect(),
		_phantom: PhantomData,
	})
}

/// Gets the directory for an instance
pub fn get_instance_dir(instance: &str) -> anyhow::Result<Option<PathBuf>> {
	super::interface::get_instance_dir(instance)
		.map(|x| x.map(PathBuf::from))
		.map_err(|e| anyhow!(e))
}

/// Creates a new instance and adds it to the config
pub fn create_instance(id: &str, config: &InstanceConfig) -> anyhow::Result<()> {
	let config = serde_json::to_string(config)?;

	super::interface::create_instance(id, &config).map_err(|e| anyhow!(e))
}

/// Creates a new template and adds it to the config
pub fn create_template(id: &str, config: &TemplateConfig) -> anyhow::Result<()> {
	let config = serde_json::to_string(config)?;

	super::interface::create_template(id, &config).map_err(|e| anyhow!(e))
}

/// Launches an instance in the background
pub fn launch_instance(instance: &str, account: Option<&str>) -> anyhow::Result<()> {
	super::interface::launch_instance(instance, account).map_err(|e| anyhow!(e))
}

/// Map of deserialized values returned from WASM functions
pub struct WASMMap<T: DeserializeOwned> {
	map: HashMap<String, String>,
	_phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> WASMMap<T> {
	/// Gets a key from the map, returning an error if it failed to deserialize
	pub fn get<K>(&self, k: &K) -> anyhow::Result<Option<T>>
	where
		String: Borrow<K>,
		K: Eq + Hash + ?Sized,
	{
		let Some(x) = self.map.get(k) else {
			return Ok(None);
		};

		serde_json::from_str(x)
			.map(Some)
			.context("Failed to deserialize value")
	}

	/// Iterates over the pairs in the map
	pub fn iter<'this>(
		&'this self,
	) -> impl Iterator<Item = (&'this String, anyhow::Result<T>)> + 'this {
		self.map.iter().map(|(k, v)| {
			let v = serde_json::from_str(v).context("Failed to deserialize value");

			(k, v)
		})
	}
}
