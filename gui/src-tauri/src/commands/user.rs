use crate::output::LauncherOutput;
use crate::State;
use anyhow::Context;
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::user::{UserConfig, UserVariant},
	core::user::UserKind,
	plugin::PluginManager,
	plugin_crate::hook::hooks::{AddUserTypes, UserTypeInfo},
	shared::output::NoOp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_users(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<(Option<String>, HashMap<String, UserInfo>), String> {
	let data = state.data.lock().await;
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let mut config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;
	let user_ids: Vec<_> = config.users.iter_users().map(|x| x.0.clone()).collect();

	let mut users = HashMap::with_capacity(user_ids.len());
	config.users.set_offline(true);
	for id in user_ids {
		let _ = config
			.users
			.authenticate_user(&id, &state.paths.core, &state.client, &mut output)
			.await
			.context("Failed to authenticate user");

		let user = config.users.get_user(&id).expect("User should exist");

		let ty = match user.get_kind() {
			UserKind::Microsoft { .. } => UserType::Microsoft,
			UserKind::Demo => UserType::Demo,
			UserKind::Unknown(..) => UserType::Other,
		};

		let info = UserInfo {
			id: id.to_string(),
			r#type: ty,
			username: user.get_name().cloned(),
			uuid: user.get_uuid().cloned(),
		};

		users.insert(id.to_string(), info);
	}

	let current_user = data
		.current_user
		.clone()
		.filter(|x| config.users.user_exists(x));

	Ok((current_user, users))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
	pub id: String,
	pub r#type: UserType,
	pub username: Option<String>,
	pub uuid: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UserType {
	Microsoft,
	Demo,
	Other,
}

#[tauri::command]
pub async fn select_user(state: tauri::State<'_, State>, user: &str) -> Result<(), String> {
	let mut data = state.data.lock().await;

	data.current_user = Some(user.to_string());
	fmt_err(data.write(&state.paths))?;

	Ok(())
}

#[tauri::command]
pub async fn login_user(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	user: &str,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("login_user");

	let user = user.to_string();
	let paths = state.paths.clone();
	let client = state.client.clone();
	let task = async move {
		let mut output = output;
		config
			.users
			.authenticate_user(&user, &paths.core, &client, &mut output)
			.await?;

		Ok::<(), anyhow::Error>(())
	};

	state.register_task("login_user", tokio::spawn(task)).await;

	Ok(())
}

#[tauri::command]
pub async fn logout_user(state: tauri::State<'_, State>, user: &str) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(user) = config.users.get_user_mut(user) else {
		return Err("User does not exist".into());
	};

	fmt_err(user.logout(&state.paths.core))?;

	Ok(())
}

#[tauri::command]
pub async fn create_user(
	state: tauri::State<'_, State>,
	id: &str,
	kind: UserVariant,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let user = UserConfig::Simple(kind);

	let plugins = fmt_err(PluginManager::load(&state.paths, &mut NoOp).await)?;

	let modifications = vec![ConfigModification::AddUser(id.into(), user)];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &state.paths, &plugins)
			.await
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn remove_user(state: tauri::State<'_, State>, user: &str) -> Result<(), String> {
	let paths = state.paths.clone();

	logout_user(state, user).await?;

	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&paths)).context("Failed to load config"))?;

	let plugins = fmt_err(PluginManager::load(&paths, &mut NoOp).await)?;

	let modifications = vec![ConfigModification::RemoveUser(user.into())];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &paths, &plugins)
			.await
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn get_supported_user_types(
	state: tauri::State<'_, State>,
) -> Result<Vec<UserTypeInfo>, String> {
	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddUserTypes, &(), &state.paths, &mut NoOp)
			.await
			.context("Failed to get new user types from plugins"),
	)?;
	let out = fmt_err(results.flatten_all_results(&mut NoOp).await)?;

	Ok(out)
}
