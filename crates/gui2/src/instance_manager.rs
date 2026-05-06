use std::{sync::Arc, time::Duration};

use anyhow::Context;
use nitrolaunch::{
	instance::tracking::{RunningInstanceEntry, RunningInstanceRegistry},
	io::paths::Paths,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};

use crate::state::BackEvent;

/// Manager for running instances
#[derive(Clone)]
pub struct RunningInstanceManager {
	running_instance_registry: Arc<Mutex<RunningInstanceRegistry>>,
	event_tx: broadcast::Sender<BackEvent>,
}

impl RunningInstanceManager {
	pub fn new(paths: &Paths, event_tx: broadcast::Sender<BackEvent>) -> anyhow::Result<Self> {
		let out = Self {
			running_instance_registry: Arc::new(Mutex::new(
				RunningInstanceRegistry::open(paths)
					.context("Failed to open running instance registry")?,
			)),
			event_tx,
		};

		Ok(out)
	}

	/// Gets the async task to update instances
	pub async fn get_run_task(self) -> ! {
		loop {
			self.update_instances().await;
			tokio::time::sleep(Duration::from_millis(75)).await;
		}
	}

	/// Updates running instances from the running registry
	pub async fn update_instances(&self) {
		let mut lock = self.running_instance_registry.lock().await;
		let prev_hash = lock.get_entries_hash();

		let _ = lock.reread();
		lock.remove_dead_instances();

		let post_hash = lock.get_entries_hash();
		std::mem::drop(lock);

		// Only emit event if the list has changed
		if prev_hash != post_hash {
			self.emit_update_event().await;
		}
	}

	/// Kills an instance
	pub async fn kill(&self, instance: &str, account: Option<&str>) {
		let mut lock = self.running_instance_registry.lock().await;
		lock.kill_instance(instance, account);
		let _ = lock.write();
		std::mem::drop(lock);
		self.emit_update_event().await;
	}

	/// Gets an instance entry
	pub async fn get_entry(
		&self,
		instance: &str,
		account: Option<&str>,
	) -> Option<RunningInstanceEntry> {
		self.running_instance_registry
			.lock()
			.await
			.get_instance(instance, account)
			.cloned()
	}

	/// Gets the list of running instances
	pub async fn get_running_instances(&self) -> Vec<RunningInstanceEntry> {
		self.running_instance_registry
			.lock()
			.await
			.iter_entries()
			.cloned()
			.collect()
	}

	/// Sends out an event to update running instances
	pub async fn emit_update_event(&self) {
		let _ = self.event_tx.send(BackEvent::UpdateRunningInstances);
	}
}
