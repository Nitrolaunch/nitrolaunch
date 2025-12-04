use std::path::PathBuf;

use crate::commands::instance::write_instance_config;
use crate::output::LauncherOutput;
use crate::State;
use anyhow::Context;
use nitrolaunch::config::modifications::{apply_modifications_and_write, ConfigModification};
use nitrolaunch::config::Config;
use nitrolaunch::instance::{transfer, Instance};
use nitrolaunch::io::lock::Lockfile;
use nitrolaunch::plugin_crate::hook::hooks::{
	AddInstanceTransferFormats, CheckMigration, CheckMigrationResult, InstanceTransferFormat,
};
use nitrolaunch::shared::id::InstanceID;
use nitrolaunch::shared::output::NoOp;

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_instance_transfer_formats(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<Vec<InstanceTransferFormat>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let results = fmt_err(
		config
			.plugins
			.call_hook(AddInstanceTransferFormats, &(), &state.paths, &mut output)
			.await,
	)?;

	let out = fmt_err(results.flatten_all_results(&mut output).await)?;

	Ok(out)
}

#[tauri::command]
pub async fn import_instance(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	format: String,
	path: String,
	id: String,
) -> Result<(), String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("import_instance");

	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let formats = fmt_err(
		transfer::load_formats(&config.plugins, &state.paths, &mut output)
			.await
			.context("Failed to load transfer formats"),
	)?;

	let config = fmt_err(
		Instance::import(
			&id,
			&format,
			&PathBuf::from(path),
			&formats,
			&config.plugins,
			&state.paths,
			&mut output,
		)
		.await
		.context("Failed to import instance"),
	)?;

	write_instance_config(state, id, config).await
}

#[tauri::command]
pub async fn export_instance(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	format: String,
	path: String,
	id: String,
) -> Result<(), String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("export_instance");

	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let formats = fmt_err(
		transfer::load_formats(&config.plugins, &state.paths, &mut output)
			.await
			.context("Failed to load transfer formats"),
	)?;

	let Some(instance) = config.instances.get_mut(&InstanceID::from(id)) else {
		return Err("Instance does not exist".into());
	};

	let mut lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;

	fmt_err(
		instance
			.export(
				&format,
				&PathBuf::from(path),
				&formats,
				&config.plugins,
				&mut lock,
				&state.paths,
				&mut output,
			)
			.await
			.context("Failed to export instance"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn check_migration(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	format: &str,
) -> Result<Option<CheckMigrationResult>, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let result = fmt_err(
		config
			.plugins
			.call_hook(
				CheckMigration,
				&format.to_string(),
				&state.paths,
				&mut output,
			)
			.await,
	)?;
	let result = fmt_err(result.first_some(&mut output).await)?;

	Ok(result)
}

#[tauri::command]
pub async fn migrate_instances(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	format: &str,
	instances: Option<Vec<String>>,
	link: bool,
) -> Result<usize, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("migrate_instances");

	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let formats = fmt_err(
		transfer::load_formats(&config.plugins, &state.paths, &mut output)
			.await
			.context("Failed to load transfer formats"),
	)?;

	let mut lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;

	let instances = fmt_err(
		nitrolaunch::instance::transfer::migrate_instances(
			format,
			instances,
			link,
			&formats,
			&config.plugins,
			&state.paths,
			&mut lock,
			&mut output,
		)
		.await
		.context("Failed to migrate instances"),
	)?;

	let mut config =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let count = instances.len();

	let modifications: Vec<_> = instances
		.into_iter()
		.map(|(id, config)| ConfigModification::AddInstance(id.into(), config))
		.collect();

	fmt_err(
		apply_modifications_and_write(&mut config, modifications, &state.paths)
			.context("Failed to modify and write config"),
	)?;

	Ok(count)
}
