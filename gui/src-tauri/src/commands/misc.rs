use std::{path::PathBuf, time::Duration};

use crate::{data::InstanceIcon, output::LauncherOutput, State};
use anyhow::{bail, Context};
use nitrolaunch::{
	core::{
		io::json_from_file,
		net::game_files::{assets::AssetIndex, version_manifest::VersionType},
	},
	instance::{setup::setup_core, update::manager::UpdateSettings},
	plugin_crate::hook::hooks::{
		AddInstanceIcons, AddSupportedLoaders, GetLoaderVersions, GetLoaderVersionsArg,
	},
	shared::{id::InstanceID, loaders::Loader, output::NoOp, UpdateDepth},
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
	let out = fmt_err(results.flatten_all_results(&mut NoOp).await)?;

	Ok(out)
}

#[tauri::command]
pub async fn get_loader_versions(
	state: tauri::State<'_, State>,
	loader: Loader,
	minecraft_version: String,
) -> Result<Vec<String>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let arg = GetLoaderVersionsArg {
		loader,
		minecraft_version,
	};

	let results = fmt_err(
		config
			.plugins
			.call_hook(GetLoaderVersions, &arg, &state.paths, &mut NoOp)
			.await
			.context("Failed to get loader versions from plugins"),
	)?;
	let out = fmt_err(results.flatten_all_results(&mut NoOp).await)?;

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

	let mut core = fmt_err(
		setup_core(
			None,
			&UpdateSettings {
				depth: UpdateDepth::Shallow,
				offline_auth: false,
			},
			&state.client,
			&config.users,
			&config.plugins,
			&state.paths,
			&mut NoOp,
		)
		.await,
	)?;

	let version_manifest = fmt_err(core.get_version_manifest(None, &mut NoOp).await)?;

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

	let mut core = fmt_err(
		setup_core(
			None,
			&UpdateSettings {
				depth: UpdateDepth::Shallow,
				offline_auth: false,
			},
			&state.client,
			&config.users,
			&config.plugins,
			&state.paths,
			&mut output,
		)
		.await,
	)?;

	fmt_err(core.get_version_manifest(None, &mut output).await)?;

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

/// Gets banner images for an installed Minecraft version from the game assets.
///
/// Returns filesystem paths to two panorama images to be stitched left-to-right
#[tauri::command]
pub async fn get_version_banner_images(
	state: tauri::State<'_, State>,
	version: &str,
) -> Result<Option<(String, String)>, String> {
	let index_path = state
		.paths
		.internal
		.join(format!("assets/indexes/{version}.json"));

	if !index_path.exists() {
		return Ok(None);
	}

	let contents: AssetIndex =
		fmt_err(json_from_file(index_path).context("Failed to open asset index"))?;

	let pano1 = contents
		.objects
		.get("minecraft/textures/gui/title/background/panorama_0.png");
	let pano2 = contents
		.objects
		.get("minecraft/textures/gui/title/background/panorama_1.png");

	let Some(pano1) = pano1 else {
		return Ok(None);
	};

	let Some(pano2) = pano2 else {
		return Ok(None);
	};

	let path1 = state
		.paths
		.internal
		.join(format!("assets/objects/{}", pano1.get_hash_path()));
	let path2 = state
		.paths
		.internal
		.join(format!("assets/objects/{}", pano2.get_hash_path()));

	if !path1.exists() || !path2.exists() {
		return Ok(None);
	}

	Ok(Some((
		path1.to_string_lossy().to_string(),
		path2.to_string_lossy().to_string(),
	)))
}

/// Starts a long-running test task
#[tauri::command]
pub async fn test_long_running_task(state: tauri::State<'_, State>) -> Result<(), String> {
	let task = tokio::spawn(async {
		tokio::time::sleep(Duration::from_secs(3)).await;

		bail!("Error:\nerror.")
	});

	state.register_task("test_task", task).await;

	Ok(())
}

#[tauri::command]
pub async fn open_data_dir(state: tauri::State<'_, State>) -> Result<(), String> {
	let path = state.paths.data.clone();
	tokio::task::spawn_blocking(move || {
		showfile::show_path_in_file_manager(&path);
	});
	Ok(())
}

#[tauri::command]
pub async fn open_instance_dir(
	state: tauri::State<'_, State>,
	instance: &str,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(instance) = config.instances.get_mut(&InstanceID::from(instance)) else {
		return Err(format!("Instance {instance} does not exist"));
	};

	let _ = instance.ensure_dirs(&state.paths);

	let path = instance.get_dirs().get().game_dir.clone();

	tokio::task::spawn_blocking(move || {
		showfile::show_path_in_file_manager(&path);
	});

	Ok(())
}

/// Gets the list of available instance icons
#[tauri::command]
pub async fn get_available_icons(
	state: tauri::State<'_, State>,
) -> Result<Vec<InstanceIcon>, String> {
	let data = state.data.lock().await;
	let saved = data.saved_instance_icons.clone();

	let Ok(config) = load_config(&state.paths, &mut NoOp).await else {
		return Ok(saved);
	};

	let Ok(mut results) = config
		.plugins
		.call_hook(AddInstanceIcons, &(), &state.paths, &mut NoOp)
		.await
	else {
		return Ok(saved);
	};

	let mut out = Vec::with_capacity(results.len());
	while let Some(result) = results.next() {
		let Ok(result) = result.result(&mut NoOp).await else {
			continue;
		};

		out.extend(
			result
				.into_iter()
				.map(|x| InstanceIcon::File(PathBuf::from(x))),
		);
	}

	out.extend(saved);

	Ok(out)
}

/// Adds a saved instance icon
#[tauri::command]
pub async fn save_icon(state: tauri::State<'_, State>, icon: String) -> Result<(), String> {
	let mut data = state.data.lock().await;
	data.saved_instance_icons
		.push(InstanceIcon::File(icon.into()));

	fmt_err(data.write(&state.paths))?;

	Ok(())
}
