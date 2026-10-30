use std::{sync::Arc, time::Duration};

use anyhow::Context;
use nitrolaunch::{
	instance::tracking::{
		REMOTE_STATUS_UPDATE_INTERVAL, RunningInstanceEntry, RunningInstanceRegistry,
	},
	io::paths::Paths,
	plugin::PluginManager,
	shared::output::NoOp,
};
use tokio::{
	sync::{Mutex, broadcast},
	time::interval,
};

use crate::state::BackEvent;

/// Manager for running instances
#[derive(Clone)]
pub struct RunningInstanceManager {
	running_instance_registry: Arc<Mutex<RunningInstanceRegistry>>,
	event_tx: broadcast::Sender<BackEvent>,
	plugins: PluginManager,
	paths: Arc<Paths>,
}

impl RunningInstanceManager {
	pub fn new(
		paths: Arc<Paths>,
		plugins: PluginManager,
		event_tx: broadcast::Sender<BackEvent>,
	) -> anyhow::Result<Self> {
		let out = Self {
			running_instance_registry: Arc::new(Mutex::new(
				RunningInstanceRegistry::open(&paths)
					.context("Failed to open running instance registry")?,
			)),
			plugins,
			paths,
			event_tx,
		};

		Ok(out)
	}

	/// Gets the async task to update instances
	pub async fn get_run_task(self) -> () {
		let mut local_interval = interval(Duration::from_millis(75));
		let mut remote_interval = interval(REMOTE_STATUS_UPDATE_INTERVAL);
		loop {
			tokio::select! {
				_ = local_interval.tick() => {
					self.update_local().await;
				}
				_ = remote_interval.tick() => {
					self.update_remote().await;
				}
			}
		}
	}

	/// Updates running instances from the running registry
	pub async fn update_local(&self) {
		let mut lock = self.running_instance_registry.lock().await;
		let prev_hash = lock.get_entries_hash();

		let _ = lock.reread();
		let _ = lock
			.update_from_plugins(false, &self.plugins, &self.paths, &mut NoOp)
			.await;

		let post_hash = lock.get_entries_hash();
		std::mem::drop(lock);

		// Only emit event if the list has changed
		if prev_hash != post_hash {
			self.emit_update_event().await;
		}
	}

	/// Updates running instances from remote sources
	pub async fn update_remote(&self) {
		let mut lock = self.running_instance_registry.lock().await;
		let prev_hash = lock.get_entries_hash();

		let _ = lock
			.update_from_plugins(true, &self.plugins, &self.paths, &mut NoOp)
			.await;

		let post_hash = lock.get_entries_hash();
		std::mem::drop(lock);

		// Only emit event if the list has changed
		if prev_hash != post_hash {
			self.emit_update_event().await;
		}
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
