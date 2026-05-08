use std::{sync::Arc, time::Duration};

use nitrolaunch::shared::output::MessageContents;
use tokio::{
	sync::{Mutex, broadcast},
	task::JoinHandle,
};

use crate::state::BackEvent;

/// Manager for long-running tasks
pub struct TaskManager {
	tasks: Vec<RunningTask>,
	event_tx: broadcast::Sender<BackEvent>,
}

impl TaskManager {
	pub fn new(event_tx: broadcast::Sender<BackEvent>) -> Self {
		Self {
			tasks: Vec::new(),
			event_tx,
		}
	}

	/// Gets the async task to update running tasks
	pub async fn get_run_task(this: Arc<Mutex<Self>>) -> ! {
		loop {
			let mut lock = this.lock().await;

			lock.update_tasks().await;

			std::mem::drop(lock);
			tokio::time::sleep(Duration::from_millis(15)).await;
		}
	}

	/// Registers a task with the task manager
	pub fn register_task(&mut self, task_id: String, join_handle: JoinHandle<anyhow::Result<()>>) {
		self.tasks.push(RunningTask {
			id: task_id,
			join_handle: Some(join_handle),
		})
	}

	/// Updates running tasks
	pub async fn update_tasks(&mut self) {
		for task in &mut self.tasks {
			if let Some(join_handle) = task.join_handle.take() {
				if join_handle.is_finished() {
					let result = join_handle.await;
					if let Ok(Err(error)) = result {
						eprintln!("Task error: {error:?}");
						let _ = self.event_tx.send(BackEvent::OutputMessage {
							message: MessageContents::Error(format!("{error:?}")),
							task: Some(task.id.clone()),
						});
					}
				} else {
					task.join_handle = Some(join_handle);
				}
			}
		}

		self.tasks.retain(|x| x.join_handle.is_some());
	}

	/// Kills a task
	pub fn kill(&mut self, task_id: &str) {
		self.tasks.retain(|task| {
			if task.id == task_id {
				if let Some(join_handle) = &task.join_handle {
					join_handle.abort();
				}
				let _ = self
					.event_tx
					.send(BackEvent::OutputEndTask(task.id.clone()));

				println!("Task {task_id} cancelled");

				false
			} else {
				true
			}
		});
	}
}

/// A single running task
#[derive(Debug)]
struct RunningTask {
	id: String,
	join_handle: Option<JoinHandle<anyhow::Result<()>>>,
}
