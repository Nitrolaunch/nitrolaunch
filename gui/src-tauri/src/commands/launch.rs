use crate::commands::instance::update_instance_impl;
use crate::data::LauncherData;
use crate::{output::LauncherOutput, State};
use anyhow::{bail, Context};
use nitrolaunch::core::io::open_named_pipe;
use nitrolaunch::core::QuickPlayType;
use nitrolaunch::instance::launch::LaunchSettings;
use nitrolaunch::instance::tracking::RunningInstanceEntry;
use nitrolaunch::io::lock::Lockfile;
use nitrolaunch::plugin_crate::try_read::TryReadExt;
use nitrolaunch::shared::id::InstanceID;
use nitrolaunch::shared::output::NoOp;
use nitrolaunch::shared::UpdateDepth;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn launch_game(
	app_handle: tauri::AppHandle,
	state: tauri::State<'_, State>,
	instance_id: String,
	offline: bool,
	account: Option<&str>,
) -> Result<(), String> {
	// let state = Arc::new(state);
	let app_handle = Arc::new(app_handle);
	let mut output = LauncherOutput::new(state.get_output_arc(app_handle.clone()));
	output.set_task(&format!("launch_instance_{instance_id}"));

	let instance_id = InstanceID::from(instance_id);

	let stdio_paths = Arc::new(Mutex::new(None));

	let data = fmt_err(LauncherData::open(&state.paths).context("Failed to open launcher data"))?;

	let account = account.or(data.current_account.as_deref());

	fmt_err(
		launch_game_impl(
			instance_id.to_string(),
			offline,
			account,
			None,
			&state,
			app_handle,
			stdio_paths.clone(),
			output,
		)
		.await
		.context("Failed to launch game"),
	)?;

	Ok(())
}

pub async fn launch_game_impl(
	instance_id: String,
	offline: bool,
	account: Option<&str>,
	quick_play: Option<QuickPlayType>,
	state: &State,
	app: Arc<AppHandle>,
	stdio_paths: Arc<Mutex<Option<(PathBuf, Option<PathBuf>)>>>,
	mut o: LauncherOutput,
) -> anyhow::Result<()> {
	println!("Launching game!");

	let mut config = load_config(&state.paths, &state.wasm_loader, &mut o)
		.await
		.context("Failed to load config")?;
	if let Some(account) = account {
		config.accounts.choose_account(account)?;
	}

	// Check first update
	let lock = Lockfile::open(&state.paths).context("Failed to open lockfile")?;
	if !lock.has_instance_done_first_update(&instance_id) {
		if let Err(e) =
			update_instance_impl(&state, app.clone(), instance_id.clone(), UpdateDepth::Full).await
		{
			bail!("{e}");
		};

		return Ok(());
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
				pipe_stdin: false,
				quick_play,
			};
			let mut handle = instance
				.launch(&paths, &mut config.accounts, &plugins, settings, &mut o)
				.await
				.context("Failed to launch instance")?;

			o.finish_task();

			handle.silence_output(true);

			*stdio_paths.lock().await = Some((
				handle.stdout().to_owned(),
				handle.stdin().map(|x| x.to_owned()),
			));

			let update_output_task = emit_instance_stdio_changes(
				app.clone(),
				instance_id.to_string(),
				handle.stdout().to_owned(),
			);

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
			app.emit("game_finished", instance_id.to_string())?;

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
) -> Result<Vec<RunningInstanceEntry>, String> {
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
pub async fn kill_instance(
	state: tauri::State<'_, State>,
	instance: &str,
	user: Option<&str>,
) -> Result<(), String> {
	state
		.running_instances
		.get()
		.unwrap()
		.lock()
		.await
		.kill(instance, user);

	Ok(())
}

#[tauri::command]
pub async fn get_instance_output(
	state: tauri::State<'_, State>,
	instance_id: &str,
) -> Result<Option<String>, String> {
	let path = {
		let lock = state.running_instances.get().unwrap().lock().await;
		let Some(entry) = lock.get_entry(instance_id, None) else {
			return Ok(None);
		};

		let Some(path) = &entry.stdout_file else {
			return Ok(None);
		};

		state.paths.internal.join("stdio").join(path)
	};

	let contents = fmt_err(
		tokio::fs::read_to_string(path)
			.await
			.context("Failed to read output file"),
	)?;

	Ok(Some(contents))
}

#[tauri::command]
pub async fn write_instance_input(
	state: tauri::State<'_, State>,
	instance_id: &str,
	input: &str,
) -> Result<(), String> {
	let path = {
		let lock = state.running_instances.get().unwrap().lock().await;
		let Some(entry) = lock.get_entry(instance_id, None) else {
			return Ok(());
		};

		let Some(path) = &entry.stdin_file else {
			return Ok(());
		};

		state.paths.internal.join("stdio").join(path)
	};

	let mut file = fmt_err(open_named_pipe(path))?;
	fmt_err(file.write_all(input.as_bytes()))?;

	Ok(())
}

#[tauri::command]
pub async fn get_instance_logs(
	state: tauri::State<'_, State>,
	instance_id: &str,
) -> Result<Vec<String>, String> {
	let mut config = fmt_err(load_config(&state.paths, &state.wasm_loader, &mut NoOp).await)?;

	let Some(instance) = config.instances.get_mut(instance_id) else {
		return Err("Instance does not exist".into());
	};

	fmt_err(
		instance
			.get_logs(&config.plugins, &state.paths, &mut NoOp)
			.await,
	)
}

#[tauri::command]
pub async fn get_instance_log(
	state: tauri::State<'_, State>,
	instance_id: &str,
	log_id: &str,
) -> Result<String, String> {
	let mut config = fmt_err(load_config(&state.paths, &state.wasm_loader, &mut NoOp).await)?;

	let Some(instance) = config.instances.get_mut(instance_id) else {
		return Err("Instance does not exist".into());
	};

	fmt_err(
		instance
			.get_log(log_id, &config.plugins, &state.paths, &mut NoOp)
			.await,
	)
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
				let _ = app.emit("update_instance_stdio", &instance_id);
			}
		}

		tokio::time::sleep(Duration::from_millis(1)).await;
	}
}
