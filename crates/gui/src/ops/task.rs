use std::{fmt::Display, sync::Arc, time::Duration};

use nitrolaunch::shared::output::{Message, MessageContents, MessageLevel};
use tokio::{
	sync::{Mutex, broadcast, mpsc},
	task::JoinHandle,
};

use crate::{simple_mutation, state::BackEvent};

/// Manager for long-running tasks
pub struct TaskManager {
	tasks: Vec<RunningTask>,
	event_tx: broadcast::Sender<BackEvent>,
	logger_tx: mpsc::Sender<Message>,
}

impl TaskManager {
	pub fn new(event_tx: broadcast::Sender<BackEvent>, logger_tx: mpsc::Sender<Message>) -> Self {
		Self {
			tasks: Vec::new(),
			event_tx,
			logger_tx,
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
						let _ = self.logger_tx.try_send(Message {
							contents: MessageContents::Error(format!("{error:?}")),
							level: MessageLevel::Important,
						});
						if task.id.is_long_running() {
							let _ = self.event_tx.send(BackEvent::ErrorToast(
								task.id.failure_message(),
								Some(format!("{error:?}")),
							));
						}
						let _ = self.event_tx.send(BackEvent::OutputEndTask {
							task: task.id.clone(),
							success: false,
						});
					} else if task.id.is_long_running() {
						let _ = self
							.event_tx
							.send(BackEvent::SuccessToast(task.id.success_message()));
						let _ = self.event_tx.send(BackEvent::OutputEndTask {
							task: task.id.clone(),
							success: true,
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
	pub fn kill(&mut self, task_id: &Task) {
		self.tasks.retain(|task| {
			if task.id == *task_id {
				if let Some(join_handle) = &task.join_handle {
					join_handle.abort();
				}
				let _ = self.event_tx.send(BackEvent::OutputEndTask {
					task: task.id.clone(),
					success: false,
				});

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
	UpdateInstancePackages(String),
	UpdateInstanceModpack(String),
	DeleteInstance,
	InstallModpack,
	ImportInstance,
	ExportInstance,
	MigrateInstances,
	SearchPackages,
	FetchRemotePlugins,
	InstallPlugin,
	FetchPluginVersions,
	InstallDefaultPlugins,
	LoginAccount,
	LoginFirstAccount,
	FetchLoaderVersions,
	Opening,
	CustomAction,
}

impl Task {
	pub fn can_cancel(&self) -> bool {
		match self {
			Self::LaunchInstance(_) => true,
			Self::UpdateInstance(_) => true,
			Self::UpdateInstancePackages(_) => true,
			Self::UpdateInstanceModpack(_) => true,
			Self::DeleteInstance => false,
			Self::InstallModpack => false,
			Self::ImportInstance => false,
			Self::ExportInstance => false,
			Self::MigrateInstances => false,
			Self::SearchPackages => false,
			Self::FetchRemotePlugins => false,
			Self::InstallPlugin => false,
			Self::FetchPluginVersions => false,
			Self::InstallDefaultPlugins => true,
			Self::LoginAccount => true,
			Self::LoginFirstAccount => true,
			Self::FetchLoaderVersions => false,
			Self::Opening => false,
			Self::CustomAction => false,
		}
	}

	pub fn is_long_running(&self) -> bool {
		match self {
			Self::LaunchInstance(_) => true,
			Self::UpdateInstance(_) => true,
			Self::UpdateInstancePackages(_) => true,
			Self::UpdateInstanceModpack(_) => true,
			Self::DeleteInstance => false,
			Self::InstallModpack => true,
			Self::ImportInstance => true,
			Self::ExportInstance => true,
			Self::MigrateInstances => true,
			Self::SearchPackages => false,
			Self::FetchRemotePlugins => false,
			Self::InstallPlugin => false,
			Self::FetchPluginVersions => false,
			Self::InstallDefaultPlugins => true,
			Self::LoginAccount => true,
			Self::LoginFirstAccount => true,
			Self::FetchLoaderVersions => false,
			Self::Opening => false,
			Self::CustomAction => false,
		}
	}

	pub fn success_message(&self) -> String {
		match self {
			Self::LaunchInstance(..) => "Launched!".to_string(),
			Self::UpdateInstance(..) => "Instance updated".to_string(),
			Self::UpdateInstancePackages(..) => "Packages updated".to_string(),
			Self::UpdateInstanceModpack(..) => "Modpack updated".to_string(),
			Self::DeleteInstance => "Instance deleted".into(),
			Self::InstallModpack => "Modpack installed".into(),
			Self::ImportInstance => "Instance imported".into(),
			Self::ExportInstance => "Instance exported".into(),
			Self::MigrateInstances => "Instances migrated".into(),
			Self::SearchPackages => "Packages searched".into(),
			Self::FetchRemotePlugins => "Plugins fetched".into(),
			Self::InstallPlugin => "Plugin installed".into(),
			Self::FetchPluginVersions => "Plugin versions fetched".into(),
			Self::InstallDefaultPlugins => "Plugins installed".into(),
			Self::LoginAccount => "Logged in".into(),
			Self::LoginFirstAccount => "Logged in".into(),
			Self::FetchLoaderVersions => "Loader versions fetched".into(),
			Self::Opening => "Opened".into(),
			Self::CustomAction => "Action completed".into(),
		}
	}

	pub fn failure_message(&self) -> String {
		match self {
			Self::LaunchInstance(id) => format!("Failed to launch instance {id}"),
			Self::UpdateInstance(id) => format!("Failed to update instance {id}"),
			Self::UpdateInstancePackages(id) => format!("Failed to update packages for {id}"),
			Self::UpdateInstanceModpack(id) => format!("Failed to update modpack for {id}"),
			Self::DeleteInstance => "Failed to delete instance".into(),
			Self::InstallModpack => "Failed to install modpack".into(),
			Self::ImportInstance => "Failed to import instance".into(),
			Self::ExportInstance => "Failed to export instance".into(),
			Self::MigrateInstances => "Failed to migrate instances".into(),
			Self::SearchPackages => "Failed to search packages".into(),
			Self::FetchRemotePlugins => "Failed to fetch plugins".into(),
			Self::InstallPlugin => "Failed to install plugin".into(),
			Self::FetchPluginVersions => "Failed to fetch plugin versions".into(),
			Self::InstallDefaultPlugins => "Failed to install plugins".into(),
			Self::LoginAccount => "Failed to log in".into(),
			Self::LoginFirstAccount => "Failed to log in".into(),
			Self::FetchLoaderVersions => "Failed to fetch loader versions".into(),
			Self::Opening => "Failed to open".into(),
			Self::CustomAction => "Failed to run action".into(),
		}
	}
}

impl Display for Task {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::LaunchInstance(id) => write!(f, "Launching instance {id}"),
			Self::UpdateInstance(id) => write!(f, "Updating instance {id}"),
			Self::UpdateInstancePackages(id) => write!(f, "Updating packages for {id}"),
			Self::UpdateInstanceModpack(id) => write!(f, "Updating modpack for {id}"),
			Self::DeleteInstance => write!(f, "Deleting instance"),
			Self::InstallModpack => write!(f, "Installing modpack"),
			Self::ImportInstance => write!(f, "Importing instance"),
			Self::ExportInstance => write!(f, "Exporting instance"),
			Self::MigrateInstances => write!(f, "Migrating instances"),
			Self::SearchPackages => write!(f, "Searching packages"),
			Self::FetchRemotePlugins => write!(f, "Fetching plugins"),
			Self::InstallPlugin => write!(f, "Installing plugin"),
			Self::FetchPluginVersions => write!(f, "Fetching plugin versions"),
			Self::InstallDefaultPlugins => write!(f, "Installing plugins"),
			Self::LoginAccount => write!(f, "Logging in"),
			Self::LoginFirstAccount => write!(f, "Logging in"),
			Self::FetchLoaderVersions => write!(f, "Fetching loader versions"),
			Self::Opening => write!(f, "Opening"),
			Self::CustomAction => write!(f, "Running"),
		}
	}
}
