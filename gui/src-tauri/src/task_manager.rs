use std::{sync::Arc, time::Duration};

use tauri::{AppHandle, Emitter};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::output::{MessageEvent, MessageType};

/// Manager for long-running tasks
pub struct TaskManager {
	tasks: Vec<RunningTask>,
	app_handle: AppHandle,
}

impl TaskManager {
	pub fn new(app_handle: AppHandle) -> Self {
		Self {
			tasks: Vec::new(),
			app_handle,
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
						eprintln!("Error: {error:?}");
						let _ = self.app_handle.emit(
							"nitro_output_message",
							MessageEvent {
								message: format!("{error:?}"),
								ty: MessageType::Error,
								task: Some(task.id.clone()),
							},
						);
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
				let _ = self.app_handle.emit("nitro_output_finish_task", task_id);

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
