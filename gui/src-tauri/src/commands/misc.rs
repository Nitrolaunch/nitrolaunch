use crate::State;
use anyhow::Context;
use mcvm::{
	core::{net::game_files::version_manifest::VersionType, util::versions::MinecraftVersion},
	instance::update::manager::UpdateManager,
	plugin_crate::hooks::AddSupportedLoaders,
	shared::{later::Later, loaders::Loader, output::NoOp, UpdateDepth},
};

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_supported_loaders(
	state: tauri::State<'_, State>,
) -> Result<Vec<Loader>, String> {
	let config = fmt_err(load_config(&state.paths, &mut NoOp).context("Failed to load config"))?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddSupportedLoaders, &(), &state.paths, &mut NoOp)
			.context("Failed to get supported loaders from plugins"),
	)?;
	let mut out = Vec::with_capacity(results.len());
	for result in results {
		let result = fmt_err(result.result(&mut NoOp))?;
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
	let config = fmt_err(load_config(&state.paths, &mut NoOp).context("Failed to load config"))?;

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
