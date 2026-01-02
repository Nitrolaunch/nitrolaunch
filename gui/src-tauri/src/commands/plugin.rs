use crate::output::LauncherOutput;
use crate::State;
use anyhow::Context;
use itertools::Itertools;
use nitrolaunch::plugin::PluginManager;
use nitrolaunch::plugin_crate::hook::hooks::{
	AddDropdownButtons, AddInstanceTiles, AddSidebarButtons, AddThemes, CustomAction,
	CustomActionArg, DropdownButton, DropdownButtonLocation, GetPage, InjectPageScript,
	InjectPageScriptArg, InstanceTile, SidebarButton, Theme,
};
use nitrolaunch::plugin_crate::plugin::PluginMetadata;
use nitrolaunch::{plugin::install::get_verified_plugins, shared::output::NoOp};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_local_plugins(state: tauri::State<'_, State>) -> Result<Vec<PluginInfo>, String> {
	let config =
		fmt_err(PluginManager::open_config(&state.paths).context("Failed to open plugin config"))?;

	let plugins = fmt_err(
		PluginManager::get_available_plugins(&state.paths)
			.context("Failed to get available plugins"),
	)?;

	let plugins = plugins.into_iter().filter_map(|x| {
		let id = x.0;
		let manifest = PluginManager::read_plugin_manifest(&id, &state.paths).ok()?;

		Some(PluginInfo {
			enabled: config.plugins.contains(&id),
			id,
			version: manifest.version,
			meta: manifest.meta,
			installed: true,
			is_official: false,
		})
	});

	Ok(plugins.sorted_by_cached_key(|x| x.id.clone()).collect())
}

#[tauri::command]
pub async fn get_remote_plugins(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	offline: bool,
) -> Result<Vec<PluginInfo>, String> {
	let verified_plugins = if offline {
		fmt_err(
			get_verified_plugins(&state.client, true)
				.await
				.context("Failed to get verified plugins"),
		)?
	} else {
		let mut output = LauncherOutput::new(state.get_output(app_handle));
		output.set_task("get_plugins");

		let verified_plugins = fmt_err(
			get_verified_plugins(&state.client, false)
				.await
				.context("Failed to get verified plugins"),
		)?;

		verified_plugins
	};

	let verified_plugins = verified_plugins.into_values().map(|x| PluginInfo {
		id: x.id,
		meta: x.meta,
		version: x.version,
		enabled: false,
		installed: false,
		is_official: x.github_owner == "Nitrolaunch",
	});

	Ok(verified_plugins
		.sorted_by_cached_key(|x| x.id.clone())
		.collect())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginInfo {
	pub id: String,
	pub version: Option<String>,
	#[serde(flatten)]
	pub meta: PluginMetadata,
	pub enabled: bool,
	pub installed: bool,
	/// Whether this is an official Nitrolaunch plugin
	pub is_official: bool,
}

#[tauri::command]
pub async fn enable_disable_plugin(
	state: tauri::State<'_, State>,
	plugin: &str,
	enabled: bool,
) -> Result<(), String> {
	if enabled {
		fmt_err(
			PluginManager::enable_plugin(plugin, &state.paths).context("Failed to enable plugin"),
		)?;
	} else {
		fmt_err(
			PluginManager::disable_plugin(plugin, &state.paths).context("Failed to disable plugin"),
		)?;
	}

	Ok(())
}

#[tauri::command]
pub async fn install_plugin(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	plugin: &str,
	version: Option<&str>,
) -> Result<(), String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("install_plugins");

	let verified_list = fmt_err(
		get_verified_plugins(&state.client, false)
			.await
			.context("Failed to get verified plugin list"),
	)?;

	let Some(plugin) = verified_list.get(plugin) else {
		return Err(format!("Unknown plugin '{plugin}'"));
	};

	fmt_err(
		plugin
			.install(version, &state.paths, &state.client, &mut NoOp)
			.await
			.context("Failed to install plugin"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn get_plugin_versions(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	plugin: &str,
) -> Result<Vec<String>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("get_plugin_versions");

	let verified_list = fmt_err(
		get_verified_plugins(&state.client, true)
			.await
			.context("Failed to get verified plugin list"),
	)?;

	let Some(plugin) = verified_list.get(plugin) else {
		return Err(format!("Unknown plugin '{plugin}'"));
	};

	let assets = fmt_err(
		plugin
			.get_candidate_assets(None, &state.client)
			.await
			.context("Failed to install plugin"),
	)?;

	let versions = assets.into_iter().map(|x| x.version).unique().collect();

	Ok(versions)
}

#[tauri::command]
pub async fn uninstall_plugin(state: tauri::State<'_, State>, plugin: &str) -> Result<(), String> {
	fmt_err(
		PluginManager::uninstall_plugin(plugin, &state.paths).context("Failed to uninstall plugin"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn install_default_plugins(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<(), String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("install_plugins");

	let default_plugins = [
		"fabric_quilt",
		"modrinth",
		"smithed",
		"stats",
		"docs",
		"multimc_transfer",
		"xmcl_transfer",
	];

	let verified_list = fmt_err(
		get_verified_plugins(&state.client, false)
			.await
			.context("Failed to get verified plugin list"),
	)?;

	for plugin in default_plugins {
		let Some(plugin) = verified_list.get(plugin) else {
			return Err(format!("Unknown plugin '{plugin}'"));
		};

		fmt_err(
			plugin
				.install(None, &state.paths, &state.client, &mut NoOp)
				.await
				.with_context(|| format!("Failed to install plugin {}", plugin.id)),
		)?;

		let _ = PluginManager::enable_plugin(&plugin.id, &state.paths);
	}

	Ok(())
}

#[tauri::command]
pub async fn get_page_inject_script(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	page: String,
	object: Option<String>,
) -> Result<Option<String>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let arg = InjectPageScriptArg { page, object };
	let mut results = fmt_err(
		config
			.plugins
			.call_hook(InjectPageScript, &arg, &state.paths, &mut output)
			.await,
	)?;

	let mut out = String::new();
	while let Some(result) = fmt_err(results.next_result(&mut output).await)? {
		out.push_str(&result);
		out.push('\n');
	}

	Ok(Some(out))
}

#[tauri::command]
pub async fn get_sidebar_buttons(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<Vec<SidebarButton>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddSidebarButtons, &(), &state.paths, &mut output)
			.await,
	)?;
	let out = fmt_err(results.flatten_all_results(&mut output).await)?;

	Ok(out)
}

#[tauri::command]
pub async fn get_plugin_page(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	page: &str,
) -> Result<Option<String>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut results = fmt_err(
		config
			.plugins
			.call_hook(GetPage, &page.to_string(), &state.paths, &mut output)
			.await,
	)?;

	while let Some(result) = fmt_err(results.next_result(&mut output).await)? {
		if let Some(result) = result {
			return Ok(Some(result));
		}
	}

	Ok(None)
}

#[tauri::command]
pub async fn get_themes(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<Vec<Theme>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddThemes, &(), &state.paths, &mut output)
			.await,
	)?;

	let out = fmt_err(results.flatten_all_results(&mut output).await)?;

	Ok(out)
}

#[tauri::command]
pub async fn run_custom_action(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	plugin: &str,
	action: String,
	payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let result = fmt_err(
		config
			.plugins
			.call_hook_on_plugin(
				CustomAction,
				plugin,
				&CustomActionArg {
					id: action,
					payload,
				},
				&state.paths,
				&mut output,
			)
			.await
			.context("Failed to call custom action"),
	)?;

	let Some(result) = result else {
		return Err("Plugin did not return a result".into());
	};

	let result = fmt_err(result.result(&mut output).await)?;

	Ok(result)
}

#[tauri::command]
pub async fn get_dropdown_buttons(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	location: DropdownButtonLocation,
) -> Result<Vec<DropdownButton>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut results = fmt_err(
		config
			.plugins
			.call_hook(AddDropdownButtons, &(), &state.paths, &mut output)
			.await,
	)?;

	let mut out = Vec::new();
	while let Some(result) = fmt_err(results.next_result(&mut output).await)? {
		out.extend(result.into_iter().filter(|x| x.location == location));
	}

	Ok(out)
}

#[tauri::command]
pub async fn get_instance_tiles(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	instance_id: String,
) -> Result<Vec<InstanceTile>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &state.wasm_loader, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddInstanceTiles, &instance_id, &state.paths, &mut output)
			.await,
	)?;

	let out = fmt_err(results.flatten_all_results(&mut output).await)?;

	Ok(out)
}
