use crate::data::{InstanceLaunch, LauncherData};
use crate::{output::LauncherOutput, State};
use crate::{RunState, RunningInstance};
use anyhow::Context;
use itertools::Itertools;
use mcvm::instance::launch::LaunchSettings;
use mcvm::plugin_crate::try_read::TryReadExt;
use mcvm::shared::id::InstanceID;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::instance::InstanceInfo;
use super::{fmt_err, load_config};

#[tauri::command]
pub async fn launch_game(
	app_handle: tauri::AppHandle,
	state: tauri::State<'_, State>,
	instance_id: String,
	offline: bool,
	user: Option<&str>,
) -> Result<(), String> {
	let state = Arc::new(state);
	let app_handle = Arc::new(app_handle);
	let mut output = LauncherOutput::new(state.get_output_arc(app_handle.clone()));
	output.set_task(&format!("launch_instance_{instance_id}"));

	let instance_id = InstanceID::from(instance_id);

	// Make sure the game is stopped first
	stop_game_impl(&state, &instance_id).await?;

	let stdio_paths = Arc::new(Mutex::new(None));

	let launched_game = fmt_err(
		get_launched_game(
			instance_id.to_string(),
			offline,
			user,
			state.clone(),
			app_handle,
			stdio_paths.clone(),
			state.data.clone(),
			output,
		)
		.await
		.context("Failed to launch game"),
	)?;
	let mut lock = state.launched_games.lock().await;
	let running_instance = RunningInstance {
		id: instance_id.clone(),
		task: launched_game,
		state: RunState::NotStarted,
		stdio_paths,
	};
	lock.insert(instance_id, running_instance);

	Ok(())
}

async fn get_launched_game(
	instance_id: String,
	offline: bool,
	user: Option<&str>,
	state: Arc<tauri::State<'_, State>>,
	app: Arc<AppHandle>,
	stdio_paths: Arc<Mutex<Option<(PathBuf, PathBuf)>>>,
	data: Arc<Mutex<LauncherData>>,
	mut o: LauncherOutput,
) -> anyhow::Result<JoinHandle<anyhow::Result<()>>> {
	println!("Launching game!");

	let mut config = load_config(&state.paths, &mut o)
		.await
		.context("Failed to load config")?;
	if let Some(user) = user {
		config.users.choose_user(user)?;
	}

	let paths = state.paths.clone();
	let plugins = config.plugins.clone();
	let instance_id = InstanceID::from(instance_id);
	o.set_instance(instance_id.clone());

	let launch_task = {
		let instance_id = instance_id.clone();
		tokio::spawn(async move {
			let mut o = o;

			let instance = config
				.instances
				.get_mut(&instance_id)
				.context("Instance does not exist")?;
			let settings = LaunchSettings {
				ms_client_id: crate::get_ms_client_id(),
				offline_auth: offline,
			};
			let mut handle = instance
				.launch(&paths, &mut config.users, &plugins, settings, &mut o)
				.await
				.context("Failed to launch instance")?;

			o.finish_task();

			handle.silence_output(true);

			// Record the launch
			let mut data = data.lock().await;
			data.last_launches.insert(
				instance_id.to_string(),
				InstanceLaunch {
					stdout: handle.stdout().to_string_lossy().to_string(),
				},
			);
			let _ = data.write(&paths);
			*stdio_paths.lock().await = Some((handle.stdout(), handle.stdin()));

			let update_output_task =
				emit_instance_stdio_changes(app.clone(), instance_id.to_string(), handle.stdout());

			let launch_task = {
				let paths = paths.clone();
				let plugins = plugins.clone();
				async move { handle.wait(&plugins, &paths, &mut o).await }
			};

			tokio::select! {
				result = launch_task => {
					result.context("Failed to wait for instance to finish")?;
				}
				_ = update_output_task => {}
			}

			println!("Game closed");
			app.emit_all("game_finished", instance_id.to_string())?;

			Ok::<(), anyhow::Error>(())
		})
	};

	Ok(launch_task)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateRunStateEvent {
	pub instance: String,
	pub state: RunState,
}

#[tauri::command]
pub async fn stop_game(mut state: tauri::State<'_, State>, instance: String) -> Result<(), String> {
	println!("Stopping game...");
	stop_game_impl(&mut state, &instance.into()).await?;

	Ok(())
}

async fn stop_game_impl(
	state: &tauri::State<'_, State>,
	instance: &InstanceID,
) -> Result<(), String> {
	let mut lock = state.launched_games.lock().await;
	if let Some(instance) = lock.get_mut(instance) {
		instance.task.abort();
	}
	lock.remove(instance);

	Ok(())
}

#[tauri::command]
pub async fn get_running_instances(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<Vec<RunningInstanceInfo>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	let config = fmt_err(
		load_config(&state.paths, &mut output)
			.await
			.context("Failed to load config"),
	)?;

	let data = state.data.lock().await;
	let launched_games = state.launched_games.lock().await;

	let instances = launched_games
		.iter()
		.sorted_by_key(|x| x.0)
		.filter_map(|(id, instance)| {
			let configured_instance = config.instances.get(id);
			let Some(configured_instance) = configured_instance else {
				return None;
			};
			let id = id.to_string();
			Some(RunningInstanceInfo {
				info: InstanceInfo {
					icon: data.instance_icons.get(&id).cloned(),
					pinned: data.pinned.contains(&id),
					id,
					name: configured_instance.get_config().name.clone(),
					side: Some(configured_instance.get_side()),
				},
				state: instance.state,
			})
		})
		.collect();

	Ok(instances)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunningInstanceInfo {
	pub info: InstanceInfo,
	pub state: RunState,
}

#[tauri::command]
pub async fn set_running_instance_state(
	state: tauri::State<'_, State>,
	instance: String,
	run_state: RunState,
) -> Result<(), String> {
	if let Some(instance) = state
		.launched_games
		.lock()
		.await
		.get_mut(&InstanceID::from(instance))
	{
		instance.state = run_state;
	}

	Ok(())
}

#[tauri::command]
pub async fn answer_password_prompt(
	state: tauri::State<'_, State>,
	answer: String,
) -> Result<(), String> {
	*state.password_prompt.lock().await = Some(answer);

	Ok(())
}

#[tauri::command]
pub async fn get_instance_output(
	state: tauri::State<'_, State>,
	instance_id: &str,
) -> Result<Option<String>, String> {
	let path = {
		let data = state.data.lock().await;
		if let Some(last_launch) = data.last_launches.get(instance_id) {
			PathBuf::from(&last_launch.stdout)
		} else {
			let lock = state.launched_games.lock().await;
			let Some(game) = lock.get(instance_id) else {
				return Ok(None);
			};

			let Some(paths) = game.stdio_paths.lock().await.clone() else {
				return Ok(None);
			};

			paths.0
		}
	};

	let contents = fmt_err(
		tokio::fs::read_to_string(path)
			.await
			.context("Failed to read output file"),
	)?;

	Ok(Some(contents))
}

async fn emit_instance_stdio_changes(
	app: Arc<AppHandle>,
	instance_id: String,
	stdout_path: PathBuf,
) -> anyhow::Result<()> {
	let mut file = tokio::fs::File::open(stdout_path).await?;
	let mut buf = [0u8; 512];

	loop {
		if let Ok(Some(bytes_read)) = file.try_read(&mut buf).await {
			if bytes_read > 0 {
				let _ = app.emit_all("update_instance_stdio", &instance_id);
			}
		}

		tokio::time::sleep(Duration::from_millis(1)).await;
	}
}
