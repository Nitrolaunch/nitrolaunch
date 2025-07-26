use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::Context;
use nitrolaunch::{instance::tracking::RunningInstanceRegistry, io::paths::Paths};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

/// Manager for running instances
pub struct RunningInstanceManager {
	running_instance_registry: RunningInstanceRegistry,
	app_handle: AppHandle,
}

impl RunningInstanceManager {
	pub fn new(paths: &Paths, app_handle: AppHandle) -> anyhow::Result<Self> {
		let mut out = Self {
			running_instance_registry: RunningInstanceRegistry::open(paths)
				.context("Failed to open running instance registry")?,
			app_handle,
		};

		out.update_instances();

		Ok(out)
	}

	/// Gets the async task to update instances
	pub async fn get_run_task(this: Arc<Mutex<Self>>) -> ! {
		loop {
			let mut lock = this.lock().await;

			lock.update_instances();

			std::mem::drop(lock);
			tokio::time::sleep(Duration::from_millis(75)).await;
		}
	}

	/// Updates running instances from the running registry
	pub fn update_instances(&mut self) {
		let prev_hash = self.running_instance_registry.get_entries_hash();

		let _ = self.running_instance_registry.reread();
		self.running_instance_registry.remove_dead_instances();

		let post_hash = self.running_instance_registry.get_entries_hash();

		// Only emit event if the list has changed
		if prev_hash != post_hash {
			self.emit_update_event();
		}
	}

	/// Kills an instance
	pub fn kill(&mut self, instance: &str) {
		self.running_instance_registry.kill_instance(instance);
		let _ = self.running_instance_registry.write();
	}

	/// Gets the list of running instance IDs
	pub fn get_running_instances(&self) -> HashSet<String> {
		self.running_instance_registry
			.iter_entries()
			.map(|x| x.instance_id.clone())
			.collect()
	}

	/// Sends out an event to update running instances
	pub fn emit_update_event(&self) {
		let _ = self.app_handle.emit_all(
			"nitro_update_running_instances",
			RunningInstancesEvent {
				running_instances: self.get_running_instances(),
			},
		);
	}
}

/// Event data for updating running instances
#[derive(Serialize, Deserialize, Clone)]
pub struct RunningInstancesEvent {
	running_instances: HashSet<String>,
}
