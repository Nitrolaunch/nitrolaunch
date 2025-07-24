// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Commands for Tauri
mod commands;
/// Storage and reading for GUI-specific data
mod data;
/// Nitrolaunch output for the launcher frontend
mod output;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::Context;
use commands::launch::UpdateRunStateEvent;
use data::LauncherData;
use nitrolaunch::core::auth_crate::mc::ClientId;
use nitrolaunch::core::{net::download::Client, user::UserManager};
use nitrolaunch::io::paths::Paths;
use nitrolaunch::shared::id::InstanceID;
use output::{OutputInner, PromptResponse};
use serde::{Deserialize, Serialize};
use tauri::api::process::restart;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager};
use tokio::task::JoinHandle;

use crate::output::ResolutionErrorEvent;

fn main() {
	let state = tauri::async_runtime::block_on(async { State::new().await })
		.expect("Error when initializing application state");
	let launched_games = state.launched_games.clone();

	let data = state.data.clone();
	let paths = state.paths.clone();

	tauri::Builder::default()
		.manage(state)
		.setup(move |app| {
			app.listen_global("update_run_state", move |event| {
				let launched_games = launched_games.clone();

				tauri::async_runtime::spawn(async move {
					let payload: UpdateRunStateEvent = serde_json::from_str(
						event
							.payload()
							.expect("Update run state event should have payload"),
					)
					.expect("Failed to deserialize state update");
					let mut lock = launched_games.lock().await;
					if let Some(instance) = lock.get_mut(&InstanceID::from(payload.instance)) {
						instance.state = payload.state;
					}
				});
			});

			// Save package resolution errors so that they can be displayed on the instance
			app.listen_global("nitro_display_resolution_error", move |event| {
				let paths = paths.clone();
				let data = data.clone();
				tauri::async_runtime::spawn(async move {
					let payload: ResolutionErrorEvent =
						serde_json::from_str(event.payload().expect("Event should have payload"))
							.expect("Failed to deserialize");

					let mut data = data.lock().await;
					data.last_resolution_errors
						.insert(payload.instance, payload.error);
					let _ = data.write(&paths);
				});
			});

			let env = app.env();
			app.listen_global("manual_restart", move |_| {
				restart(&env);
			});

			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			commands::launch::launch_game,
			commands::launch::stop_game,
			commands::launch::answer_password_prompt,
			commands::instance::get_instances,
			commands::instance::get_profiles,
			commands::instance::get_instance_groups,
			commands::launch::get_running_instances,
			commands::launch::set_running_instance_state,
			commands::launch::get_instance_output,
			commands::instance::pin_instance,
			commands::instance::get_instance_config,
			commands::instance::get_editable_instance_config,
			commands::instance::get_profile_config,
			commands::instance::get_editable_profile_config,
			commands::instance::get_global_profile,
			commands::instance::write_instance_config,
			commands::instance::write_profile_config,
			commands::instance::write_global_profile,
			commands::instance::update_instance,
			commands::instance::update_instance_packages,
			commands::instance::get_instance_resolution_error,
			commands::instance::delete_instance,
			commands::instance::delete_profile,
			commands::instance::get_profile_users,
			commands::instance::get_last_opened_instance,
			commands::instance::set_last_opened_instance,
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
			commands::plugin::get_page_inject_script,
			commands::plugin::get_sidebar_buttons,
			commands::plugin::get_plugin_page,
			commands::user::get_users,
			commands::user::select_user,
			commands::user::login_user,
			commands::user::logout_user,
			commands::user::create_user,
			commands::user::remove_user,
			commands::misc::get_supported_loaders,
			commands::misc::get_minecraft_versions,
		])
		.run(tauri::generate_context!())
		.expect("Error while running tauri application");
}

/// State for the Tauri application
pub struct State {
	pub data: Arc<Mutex<LauncherData>>,
	pub launched_games: Arc<Mutex<HashMap<InstanceID, RunningInstance>>>,
	pub paths: Paths,
	pub client: Client,
	pub user_manager: Mutex<UserManager>,
	/// Map of users to their already entered passkeys
	pub passkeys: Arc<Mutex<HashMap<String, String>>>,
	pub password_prompt: PromptResponse,
	pub output_inner: OnceLock<OutputInner>,
}

impl State {
	async fn new() -> anyhow::Result<Self> {
		let paths = Paths::new().await?;
		Ok(Self {
			data: Arc::new(Mutex::new(
				LauncherData::open(&paths).context("Failed to open launcher data")?,
			)),
			launched_games: Arc::new(Mutex::new(HashMap::new())),
			paths,
			client: Client::new(),
			user_manager: Mutex::new(UserManager::new(get_ms_client_id())),
			passkeys: Arc::new(Mutex::new(HashMap::new())),
			password_prompt: PromptResponse::new(Mutex::new(None)),
			output_inner: OnceLock::new(),
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
		})
	}
}

/// A running instance
pub struct RunningInstance {
	/// The ID of the instance
	pub id: InstanceID,
	/// The tokio task for the running instance
	pub task: JoinHandle<anyhow::Result<()>>,
	/// State of the instance in it's lifecycle
	pub state: RunState,
	/// The path to the stdout and stdin files for this instance, filled once it is ready
	pub stdio_paths: Arc<Mutex<Option<(PathBuf, PathBuf)>>>,
}

/// State of a running instance
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
	NotStarted,
	Preparing,
	Running,
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
