use crate::data::{InstanceLaunch, LauncherData};
use crate::{output::LauncherOutput, State};
use anyhow::Context;
use nitrolaunch::instance::launch::LaunchSettings;
use nitrolaunch::plugin_crate::try_read::TryReadExt;
use nitrolaunch::shared::id::InstanceID;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

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

	let stdio_paths = Arc::new(Mutex::new(None));

	fmt_err(
		launch_game_impl(
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

	Ok(())
}

async fn launch_game_impl(
	instance_id: String,
	offline: bool,
	user: Option<&str>,
	state: Arc<tauri::State<'_, State>>,
	app: Arc<AppHandle>,
	stdio_paths: Arc<Mutex<Option<(PathBuf, PathBuf)>>>,
	data: Arc<Mutex<LauncherData>>,
	mut o: LauncherOutput,
) -> anyhow::Result<()> {
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

	let task = {
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
			std::mem::drop(data);

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

	state
		.register_task(&format!("launch_instance_{instance_id}"), task)
		.await;

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
pub async fn get_running_instances(
	state: tauri::State<'_, State>,
) -> Result<HashSet<String>, String> {
	Ok(state
		.running_instances
		.get()
		.unwrap()
		.lock()
		.await
		.get_running_instances())
}

#[tauri::command]
pub async fn update_running_instances(state: tauri::State<'_, State>) -> Result<(), String> {
	state
		.running_instances
		.get()
		.unwrap()
		.lock()
		.await
		.emit_update_event();

	Ok(())
}

#[tauri::command]
pub async fn kill_instance(state: tauri::State<'_, State>, instance: &str) -> Result<(), String> {
	state
		.running_instances
		.get()
		.unwrap()
		.lock()
		.await
		.kill(instance);

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
			return Ok(None);
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
