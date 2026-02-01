use crate::output::LauncherOutput;
use crate::State;
use anyhow::Context;
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::account::{AccountConfig, AccountVariant},
	core::account::AccountKind,
	plugin::PluginManager,
	plugin_crate::hook::hooks::{AddAccountTypes, AccountTypeInfo},
	shared::output::NoOp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_accounts(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<(Option<String>, HashMap<String, AccountInfo>), String> {
	let data = state.data.lock().await;
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let mut config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;
	let account_ids: Vec<_> = config.accounts.iter_accounts().map(|x| x.0.clone()).collect();

	let mut accounts = HashMap::with_capacity(account_ids.len());
	config.accounts.set_offline(true);
	for id in account_ids {
		let _ = config
			.accounts
			.authenticate_account(&id, &state.paths.core, &state.client, &mut output)
			.await
			.context("Failed to authenticate account");

		let account = config.accounts.get_account(&id).expect("Account should exist");

		let ty = match account.get_kind() {
			AccountKind::Microsoft { .. } => AccountType::Microsoft,
			AccountKind::Demo => AccountType::Demo,
			AccountKind::Unknown(..) => AccountType::Other,
		};

		let info = AccountInfo {
			id: id.to_string(),
			r#type: ty,
			username: account.get_name().cloned(),
			uuid: account.get_uuid().cloned(),
		};

		accounts.insert(id.to_string(), info);
	}

	let current_account = data
		.current_account
		.clone()
		.filter(|x| config.accounts.account_exists(x));

	Ok((current_account, accounts))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountInfo {
	pub id: String,
	pub r#type: AccountType,
	pub username: Option<String>,
	pub uuid: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AccountType {
	Microsoft,
	Demo,
	Other,
}

#[tauri::command]
pub async fn select_account(state: tauri::State<'_, State>, account: &str) -> Result<(), String> {
	let mut data = state.data.lock().await;

	data.current_account = Some(account.to_string());
	fmt_err(data.write(&state.paths))?;

	Ok(())
}

#[tauri::command]
pub async fn login_account(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	account: &str,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("login_account");

	let account = account.to_string();
	let paths = state.paths.clone();
	let client = state.client.clone();
	let task = async move {
		let mut output = output;
		config
			.accounts
			.authenticate_account(&account, &paths.core, &client, &mut output)
			.await?;

		Ok::<(), anyhow::Error>(())
	};

	state.register_task("login_account", tokio::spawn(task)).await;

	Ok(())
}

#[tauri::command]
pub async fn logout_account(state: tauri::State<'_, State>, account: &str) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(account) = config.accounts.get_account_mut(account) else {
		return Err("Account does not exist".into());
	};

	fmt_err(account.logout(&state.paths.core))?;

	Ok(())
}

#[tauri::command]
pub async fn create_account(
	state: tauri::State<'_, State>,
	id: &str,
	kind: AccountVariant,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let account = AccountConfig::Simple(kind);

	let plugins = fmt_err(PluginManager::load(&state.paths, &mut NoOp).await)?;

	let modifications = vec![ConfigModification::AddAccount(id.into(), account)];
	fmt_err(
		apply_modifications_and_write(
			&mut configuration,
			modifications,
			&state.paths,
			&plugins,
			&mut NoOp,
		)
		.await
		.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn remove_account(state: tauri::State<'_, State>, account: &str) -> Result<(), String> {
	let paths = state.paths.clone();

	logout_account(state, account).await?;

	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&paths)).context("Failed to load config"))?;

	let plugins = fmt_err(PluginManager::load(&paths, &mut NoOp).await)?;

	let modifications = vec![ConfigModification::RemoveAccount(account.into())];
	fmt_err(
		apply_modifications_and_write(
			&mut configuration,
			modifications,
			&paths,
			&plugins,
			&mut NoOp,
		)
		.await
		.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn get_supported_account_types(
	state: tauri::State<'_, State>,
) -> Result<Vec<AccountTypeInfo>, String> {
	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddAccountTypes, &(), &state.paths, &mut NoOp)
			.await
			.context("Failed to get new account types from plugins"),
	)?;
	let out = fmt_err(results.flatten_all_results(&mut NoOp).await)?;

	Ok(out)
}
