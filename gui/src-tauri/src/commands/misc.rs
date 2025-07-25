use crate::{output::LauncherOutput, State};
use anyhow::Context;
use nitrolaunch::{
	core::{net::game_files::version_manifest::VersionType, util::versions::MinecraftVersion},
	instance::update::manager::UpdateManager,
	plugin_crate::hooks::AddSupportedLoaders,
	shared::{later::Later, loaders::Loader, output::NoOp, UpdateDepth},
};

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_supported_loaders(state: tauri::State<'_, State>) -> Result<Vec<Loader>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddSupportedLoaders, &(), &state.paths, &mut NoOp)
			.await
			.context("Failed to get supported loaders from plugins"),
	)?;
	let mut out = Vec::with_capacity(results.len());
	for result in results {
		let result = fmt_err(result.result(&mut NoOp).await)?;
		out.extend(result);
	}

	Ok(out)
}

/// Get a list of all Minecraft versions, including from plugins
#[tauri::command]
pub async fn get_minecraft_versions(
	state: tauri::State<'_, State>,
	releases_only: bool,
) -> Result<Vec<String>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	// Use the UpdateManager and then take the version info from it
	let mut manager = UpdateManager::new(UpdateDepth::Shallow);
	manager.set_version(&MinecraftVersion::Latest);
	fmt_err(
		manager
			.fulfill_requirements(
				&config.users,
				&config.plugins,
				&state.paths,
				&state.client,
				&mut NoOp,
			)
			.await,
	)?;

	let Later::Full(version_manifest) = manager.version_manifest else {
		return Err("Version manifest not fulfilled".into());
	};

	if releases_only {
		Ok(version_manifest
			.manifest
			.versions
			.iter()
			.filter_map(|x| {
				if let VersionType::Release = &x.ty {
					Some(x.id.clone())
				} else {
					None
				}
			})
			.rev()
			.collect())
	} else {
		Ok(version_manifest.list.clone())
	}
}

/// Updates the version manifest
#[tauri::command]
pub async fn update_version_manifest(
	app_handle: tauri::AppHandle,
	state: &State,
) -> Result<(), String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("update_versions");

	// Use the UpdateManager and then take the version info from it
	let mut manager = UpdateManager::new(UpdateDepth::Full);
	manager.set_version(&MinecraftVersion::Latest);
	fmt_err(
		manager
			.fulfill_requirements(
				&config.users,
				&config.plugins,
				&state.paths,
				&state.client,
				&mut output,
			)
			.await,
	)?;

	Ok(())
}

/// Gets whether this is the first time the launcher was opened
#[tauri::command]
pub async fn get_is_first_launch(state: tauri::State<'_, State>) -> Result<bool, String> {
	let mut data = state.data.lock().await;
	let out = !data.launcher_opened_before;
	data.launcher_opened_before = true;

	fmt_err(data.write(&state.paths))?;

	Ok(out)
}
