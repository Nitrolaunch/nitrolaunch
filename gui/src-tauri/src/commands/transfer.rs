use std::path::PathBuf;

use crate::commands::instance::write_instance_config;
use crate::output::LauncherOutput;
use crate::State;
use anyhow::Context;
use nitrolaunch::instance::{transfer, Instance};
use nitrolaunch::plugin_crate::hooks::{AddInstanceTransferFormats, InstanceTransferFormat};
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

	let mut out = Vec::new();
	for result in results {
		let result = fmt_err(result.result(&mut output).await)?;
		out.extend(result);
	}

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
