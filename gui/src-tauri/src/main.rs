// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Commands for Tauri
mod commands;
/// Storage and reading for GUI-specific data
mod data;
/// Manager for running instances
mod instance_manager;
/// Nitrolaunch output for the launcher frontend
mod output;
/// Management of long-running tasks
mod task_manager;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::Context;
use data::LauncherData;
use nitrolaunch::core::auth_crate::mc::ClientId;
use nitrolaunch::core::{net::download::Client, user::UserManager};
use nitrolaunch::io::logging::Logger;
use nitrolaunch::io::paths::Paths;
use nitrolaunch::shared::output::Message;
use output::{OutputInner, PromptResponse};
use tauri::async_runtime::{Mutex, Sender};
use tauri::process::restart;
use tauri::{AppHandle, Emitter, Listener, Manager};

use crate::commands::misc::update_version_manifest;
use crate::instance_manager::RunningInstanceManager;
use crate::output::{MessageEvent, MessageType, ResolutionErrorEvent};
use crate::task_manager::TaskManager;

fn main() {
	fix_compatability();

	let (logging_tx, mut logging_rx) = tokio::sync::mpsc::channel::<Message>(100);

	let state = tauri::async_runtime::block_on(async { State::new(logging_tx).await })
		.expect("Error when initializing application state");
	let data = state.data.clone();
	let paths = state.paths.clone();

	let state2 = state.clone();

	tauri::Builder::default()
		.plugin(tauri_plugin_clipboard_manager::init())
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_updater::Builder::new().build())
		.plugin(tauri_plugin_shell::init())
		.setup(move |app| {
			// Setup logging
			let mut logger = Logger::new(&paths, "gui")?;
			tauri::async_runtime::spawn(async move {
				loop {
					if let Some(message) = logging_rx.recv().await {
						let _ = logger.log_message(message.contents, message.level);
					}

					tokio::time::sleep(Duration::from_millis(3)).await;
				}

				#[allow(unreachable_code)]
				Ok::<(), anyhow::Error>(())
			});

			// Setup task manager
			let task_manager = TaskManager::new(app.app_handle().clone());

			let _ = state2.task_manager.set(Arc::new(Mutex::new(task_manager)));

			// Update tasks periodically
			let task = TaskManager::get_run_task(state2.task_manager.get().unwrap().clone());
			tauri::async_runtime::spawn(task);

			// Setup running instance manager
			let running_instance_manager =
				RunningInstanceManager::new(&paths, app.app_handle().clone())
					.expect("Failed to setup running instance manager");

			let _ = state2
				.running_instances
				.set(Arc::new(Mutex::new(running_instance_manager)));

			// Update running instances periodically
			let task = RunningInstanceManager::get_run_task(
				state2.running_instances.get().unwrap().clone(),
			);
			tauri::async_runtime::spawn(task);

			// Perform inital start tasks
			{
				let state = state2.clone();
				let app_handle = app.app_handle().clone();
				let app_handle2 = app.app_handle().clone();
				tauri::async_runtime::spawn(async move {
					let result = update_version_manifest(app_handle, &state).await;
					if let Err(e) = result {
						let _ = app_handle2.emit(
							"nitro_output_message",
							MessageEvent {
								message: format!("{e:?}"),
								ty: MessageType::Error,
								task: None,
							},
						);
					}
				});
			}

			// Save package resolution errors so that they can be displayed on the instance
			app.listen_any("nitro_display_resolution_error", move |event| {
				let paths = paths.clone();
				let data = data.clone();
				tauri::async_runtime::spawn(async move {
					let payload: ResolutionErrorEvent =
						serde_json::from_str(event.payload()).expect("Failed to deserialize");

					let mut data = data.lock().await;
					data.last_resolution_errors
						.insert(payload.instance, payload.error);
					let _ = data.write(&paths);
				});
			});

			let env = app.env();
			app.listen_any("manual_restart", move |_| {
				restart(&env);
			});

			Ok(())
		})
		.manage(state)
		.invoke_handler(tauri::generate_handler![
			commands::launch::launch_game,
			commands::launch::answer_password_prompt,
			commands::launch::get_running_instances,
			commands::launch::update_running_instances,
			commands::launch::kill_instance,
			commands::launch::get_instance_output,
			commands::launch::write_instance_input,
			commands::instance::get_instances,
			commands::instance::get_templates,
			commands::instance::get_instance_groups,
			commands::instance::pin_instance,
			commands::instance::get_instance_config,
			commands::instance::get_editable_instance_config,
			commands::instance::get_template_config,
			commands::instance::get_editable_template_config,
			commands::instance::get_base_template,
			commands::instance::write_instance_config,
			commands::instance::write_template_config,
			commands::instance::write_base_template,
			commands::instance::update_instance,
			commands::instance::update_instance_packages,
			commands::instance::get_instance_resolution_error,
			commands::instance::delete_instance,
			commands::instance::delete_template,
			commands::instance::get_template_users,
			commands::instance::get_last_opened_instance,
			commands::instance::set_last_opened_instance,
			commands::instance::get_instance_has_updated,
			commands::package::get_packages,
			commands::package::preload_packages,
			commands::package::get_package_meta,
			commands::package::get_package_props,
			commands::package::get_package_meta_and_props,
			commands::package::get_declarative_package_contents,
			commands::package::get_package_repos,
			commands::package::get_instance_packages,
			commands::package::sync_packages,
			commands::package::get_last_selected_repo,
			commands::package::set_last_selected_repo,
			commands::package::get_last_added_package_location,
			commands::package::set_last_added_package_location,
			commands::plugin::get_local_plugins,
			commands::plugin::get_remote_plugins,
			commands::plugin::enable_disable_plugin,
			commands::plugin::install_plugin,
			commands::plugin::uninstall_plugin,
			commands::plugin::install_default_plugins,
			commands::plugin::get_page_inject_script,
			commands::plugin::get_sidebar_buttons,
			commands::plugin::get_plugin_page,
			commands::plugin::get_themes,
			commands::plugin::run_custom_action,
			commands::plugin::get_dropdown_buttons,
			commands::plugin::get_instance_tiles,
			commands::user::get_users,
			commands::user::select_user,
			commands::user::login_user,
			commands::user::logout_user,
			commands::user::create_user,
			commands::user::remove_user,
			commands::user::get_supported_user_types,
			commands::settings::get_settings,
			commands::settings::write_settings,
			commands::transfer::get_instance_transfer_formats,
			commands::transfer::import_instance,
			commands::transfer::export_instance,
			commands::transfer::check_migration,
			commands::transfer::migrate_instances,
			commands::misc::get_supported_loaders,
			commands::misc::get_loader_versions,
			commands::misc::get_minecraft_versions,
			commands::misc::get_is_first_launch,
			commands::misc::get_version_banner_images,
			commands::misc::test_long_running_task,
			commands::misc::open_data_dir,
			commands::misc::open_instance_dir,
			commands::misc::get_available_icons,
			commands::misc::save_icon,
			commands::misc::custom_scrollbar_needed,
			commands::cancel_task,
		])
		.run(tauri::generate_context!())
		.expect("Error while running tauri application");
}

/// State for the Tauri application
#[derive(Clone)]
pub struct State {
	pub data: Arc<Mutex<LauncherData>>,
	// Will be filled during setup process
	pub running_instances: Arc<OnceLock<Arc<Mutex<RunningInstanceManager>>>>,
	pub task_manager: Arc<OnceLock<Arc<Mutex<TaskManager>>>>,
	pub paths: Paths,
	pub client: Client,
	pub user_manager: Arc<Mutex<UserManager>>,
	/// Map of users to their already entered passkeys
	pub passkeys: Arc<Mutex<HashMap<String, String>>>,
	pub password_prompt: PromptResponse,
	pub output_inner: Arc<OnceLock<OutputInner>>,
	pub logging_sender: Sender<Message>,
}

impl State {
	async fn new(logging_sender: Sender<Message>) -> anyhow::Result<Self> {
		let paths = Paths::new().await?;
		Ok(Self {
			data: Arc::new(Mutex::new(
				LauncherData::open(&paths).context("Failed to open launcher data")?,
			)),
			running_instances: Arc::new(OnceLock::new()),
			task_manager: Arc::new(OnceLock::new()),
			paths,
			client: Client::new(),
			user_manager: Arc::new(Mutex::new(UserManager::new(get_ms_client_id()))),
			passkeys: Arc::new(Mutex::new(HashMap::new())),
			password_prompt: PromptResponse::new(Mutex::new(None)),
			output_inner: Arc::new(OnceLock::new()),
			logging_sender,
		})
	}

	pub fn get_output(&self, app_handle: AppHandle) -> &OutputInner {
		self.get_output_arc(Arc::new(app_handle))
	}

	pub fn get_output_arc(&self, app_handle: Arc<AppHandle>) -> &OutputInner {
		self.output_inner.get_or_init(|| OutputInner {
			app: app_handle,
			password_prompt: self.password_prompt.clone(),
			passkeys: self.passkeys.clone(),
			logger: self.logging_sender.clone(),
		})
	}

	/// Registers a long-running task with the task manager. Panics if the task manager is not set up yet
	pub async fn register_task(
		&self,
		task_id: &str,
		join_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
	) {
		self.task_manager
			.get()
			.unwrap()
			.lock()
			.await
			.register_task(task_id.to_string(), join_handle);
	}
}

/// Get the Microsoft client ID
pub fn get_ms_client_id() -> ClientId {
	ClientId::new(get_raw_ms_client_id().to_string())
}

const fn get_raw_ms_client_id() -> &'static str {
	if let Some(id) = option_env!("NITRO_MS_CLIENT_ID") {
		id
	} else {
		// Please don't use my client ID :)
		"402abc71-43fb-45c1-b230-e7fc9d4485fe"
	}
}

/// Runs functions to enhance compatability with different systems
fn fix_compatability() {
	// Fix for Wayland causing the app to crash
	if std::env::var("WAYLAND_DISPLAY").is_ok() {
		unsafe {
			std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
		}
	}
}
