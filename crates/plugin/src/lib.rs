#![warn(missing_docs)]

//! This library is used by both Nitrolaunch to load plugins, and as a framework for defining
//! Rust plugins for Nitrolaunch to use

use anyhow::{bail, Context};
use hook_call::HookHandle;
use hooks::{Hook, OnLoad};
use itertools::Itertools;
use nitro_core::Paths;
use nitro_shared::output::NitroOutput;
use plugin::{HookPriority, Plugin, DEFAULT_PROTOCOL_VERSION, NEWEST_PROTOCOL_VERSION};

/// API for Rust-based plugins to use
#[cfg(feature = "api")]
pub mod api;
/// Implementation for calling hooks
pub mod hook_call;
/// Plugin hooks and their definitions
pub mod hooks;
/// Serialized output format for plugins
pub mod input_output;
/// Plugins
pub mod plugin;
/// Tokio helpers for AsyncRead
pub mod try_read;

pub use nitro_shared as shared;

use crate::hooks::StartWorker;

/// Environment variable that debugs plugins when set
pub static PLUGIN_DEBUG_ENV: &str = "NITRO_PLUGIN_DEBUG";

/// Gets whether plugin debugging is enabled
pub fn plugin_debug_enabled() -> bool {
	std::env::var(PLUGIN_DEBUG_ENV).unwrap_or_default() == "1"
}

/// A manager for plugins that is used to call their hooks.
/// Does not handle actually loading the plugins from files
pub struct CorePluginManager {
	plugins: Vec<Plugin>,
	plugin_list: Vec<String>,
	nitro_version: Option<&'static str>,
}

impl Default for CorePluginManager {
	fn default() -> Self {
		Self::new()
	}
}

impl CorePluginManager {
	/// Construct a new PluginManager
	pub fn new() -> Self {
		Self {
			plugins: Vec::new(),
			plugin_list: Vec::new(),
			nitro_version: None,
		}
	}

	/// Set the Nitrolaunch version of the manager
	pub fn set_nitro_version(&mut self, version: &'static str) {
		self.nitro_version = Some(version);
	}

	/// Add a plugin to the manager
	pub async fn add_plugin(
		&mut self,
		mut plugin: Plugin,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		// Check the protocol version
		if plugin
			.get_manifest()
			.protocol_version
			.unwrap_or(DEFAULT_PROTOCOL_VERSION)
			> NEWEST_PROTOCOL_VERSION
		{
			bail!("Plugin has a newer protocol version than Nitrolaunch");
		}

		// Update the plugin list
		self.plugin_list.push(plugin.get_id().clone());

		// Call the on_load hook
		let result = plugin
			.call_hook(
				&OnLoad,
				&(),
				paths,
				self.nitro_version,
				&self.plugin_list,
				o,
			)
			.await
			.context("Failed to call on_load hook of plugin")?;
		if let Some(result) = result {
			result.result(o).await?;
		}

		// Call the start_worker hook
		let worker_handle = plugin
			.call_hook(
				&StartWorker,
				&(),
				paths,
				self.nitro_version,
				&self.plugin_list,
				o,
			)
			.await
			.context("Failed to call start_worker hook of plugin")?;
		if let Some(worker_handle) = worker_handle {
			plugin
				.set_worker(worker_handle)
				.await
				.context("Failed to set plugin worker")?;
		}

		self.plugins.push(plugin);

		Ok(())
	}

	/// Call a plugin hook on the manager and collects the results into a Vec
	pub async fn call_hook<H: Hook>(
		&self,
		hook: H,
		arg: &H::Arg,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Vec<HookHandle<H>>> {
		let mut out = Vec::new();
		for plugin in self.plugins.iter().sorted_by_key(|x| PluginSort {
			priority: x.get_hook_priority(&hook),
			id: x.get_id().clone(),
		}) {
			let result = plugin
				.call_hook(&hook, arg, paths, self.nitro_version, &self.plugin_list, o)
				.await
				.with_context(|| format!("Hook failed for plugin {}", plugin.get_id()))?;
			out.extend(result);
		}

		Ok(out)
	}

	/// Call a plugin hook on the manager on a specific plugin
	pub async fn call_hook_on_plugin<H: Hook>(
		&self,
		hook: H,
		plugin_id: &str,
		arg: &H::Arg,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<HookHandle<H>>> {
		for plugin in &self.plugins {
			if plugin.get_id() == plugin_id {
				let result = plugin
					.call_hook(&hook, arg, paths, self.nitro_version, &self.plugin_list, o)
					.await
					.context("Plugin hook failed")?;
				return Ok(result);
			}
		}

		bail!("No plugin found that matched the given ID")
	}

	/// Iterate over the plugins
	pub fn iter_plugins(&self) -> impl Iterator<Item = &Plugin> {
		self.plugins.iter()
	}

	/// Checks whether the given plugin is present and enabled in the manager
	pub fn has_plugin(&self, plugin_id: &str) -> bool {
		self.plugin_list.iter().any(|x| x == plugin_id)
	}
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct PluginSort {
	priority: HookPriority,
	id: String,
}
