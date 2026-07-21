use std::{fmt::Display, sync::Arc, time::Duration};

use tokio::{
	sync::{Mutex, broadcast},
	task::JoinHandle,
};

use crate::{simple_mutation, state::BackEvent};

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
	pub fn register_task(&mut self, task_id: Task, join_handle: JoinHandle<anyhow::Result<()>>) {
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
						let _ = self.event_tx.send(BackEvent::ErrorToast(
							format!("Failed {}", task.id),
							Some(format!("{error:?}")),
						));
					}
				} else {
					task.join_handle = Some(join_handle);
				}
			}
		}

		self.tasks.retain(|x| {
			if x.join_handle.is_none() {
				let _ = self.event_tx.send(BackEvent::OutputEndTask(x.id.clone()));
				false
			} else {
				true
			}
		});
	}

	pub fn is_task_running(&self, task_id: &Task) -> bool {
		self.tasks.iter().any(|task| task.id == *task_id)
	}

	/// Kills a task
	pub fn kill(&mut self, task_id: &Task) {
		self.tasks.retain(|task| {
			if task.id == *task_id {
				if let Some(join_handle) = &task.join_handle {
					join_handle.abort();
				}
				let _ = self
					.event_tx
					.send(BackEvent::OutputEndTask(task.id.clone()));

				println!("Task {task_id:?} cancelled");

				false
			} else {
				true
			}
		});
	}
}

simple_mutation!(
	name = KillTask,
	ok = (),
	err = anyhow::Error,
	keys = Task,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let task = keys.clone();

		async move {
			back_state.kill_task(&task).await;
			Ok(())
		}
	}
);

/// A single running task
#[derive(Debug)]
struct RunningTask {
	id: Task,
	join_handle: Option<JoinHandle<anyhow::Result<()>>>,
}

/// Different types of long-running tasks across the app
#[derive(PartialEq, Clone, Debug, Hash, Eq)]
pub enum Task {
	LaunchInstance(String),
	UpdateInstance(String),
	UpdateInstanceContent(String),
	DeleteInstance,
	InstallModpack,
	SearchPackages,
	FetchRemotePlugins,
	InstallPlugin,
	FetchPluginVersions,
	LoginAccount,
}

impl Display for Task {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::LaunchInstance(id) => write!(f, "Launching instance {id}"),
			Self::UpdateInstance(id) => write!(f, "Updating instance {id}"),
			Self::UpdateInstanceContent(id) => write!(f, "Updating content for {id}"),
			Self::DeleteInstance => write!(f, "Deleting instance"),
			Self::InstallModpack => write!(f, "Installing modpack"),
			Self::SearchPackages => write!(f, "Searching packages"),
			Self::FetchRemotePlugins => write!(f, "Fetching plugins"),
			Self::InstallPlugin => write!(f, "Installing plugin"),
			Self::FetchPluginVersions => write!(f, "Fetching plugin versions"),
			Self::LoginAccount => write!(f, "Logging in"),
		}
	}
}
