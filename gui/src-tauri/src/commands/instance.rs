use crate::output::{LauncherOutput, SerializableResolutionError};
use crate::State;
use anyhow::{bail, Context};
use itertools::Itertools;
use nitrolaunch::config::modifications::{apply_modifications_and_write, ConfigModification};
use nitrolaunch::config::Config;
use nitrolaunch::config_crate::instance::InstanceConfig;
use nitrolaunch::config_crate::template::TemplateConfig;
use nitrolaunch::core::io::json_to_file_pretty;
use nitrolaunch::core::util::versions::MinecraftVersion;
use nitrolaunch::instance::delete_instance_files;
use nitrolaunch::instance::setup::setup_core;
use nitrolaunch::instance::update::manager::UpdateManager;
use nitrolaunch::instance::update::InstanceUpdateContext;
use nitrolaunch::io::lock::Lockfile;
use nitrolaunch::pkg::eval::EvalConstants;
use nitrolaunch::shared::id::{InstanceID, TemplateID};
use nitrolaunch::shared::output::NoOp;
use nitrolaunch::shared::{Side, UpdateDepth};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::Emitter;

use super::{fmt_err, load_config};

#[tauri::command]
pub async fn get_instances(state: tauri::State<'_, State>) -> Result<Vec<InstanceInfo>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let data = state.data.lock().await;

	let instances = config
		.instances
		.iter()
		.sorted_by_key(|x| x.0)
		.map(|(id, instance)| {
			let id = id.to_string();
			InstanceInfo {
				icon: instance.get_config().icon.clone(),
				pinned: data.pinned.contains(&id),
				id,
				name: instance.get_config().name.clone(),
				side: Some(instance.get_side()),
				from_plugin: instance.get_config().original_config.from_plugin,
				version: Some(instance.get_config().version.to_string()),
			}
		})
		.collect();

	Ok(instances)
}

#[tauri::command]
pub async fn get_templates(state: tauri::State<'_, State>) -> Result<Vec<InstanceInfo>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let templates = config
		.templates
		.iter()
		.sorted_by_key(|x| x.0)
		.map(|(id, template)| {
			let id = id.to_string();
			InstanceInfo {
				icon: template.instance.icon.clone(),
				pinned: false,
				id,
				name: template.instance.name.clone(),
				side: template.instance.side,
				from_plugin: template.instance.from_plugin,
				version: template
					.instance
					.version
					.as_ref()
					.map(|x| MinecraftVersion::from_deser(x).to_string()),
			}
		})
		.collect();

	Ok(templates)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceInfo {
	pub id: String,
	pub name: Option<String>,
	pub side: Option<Side>,
	pub icon: Option<String>,
	pub pinned: bool,
	pub from_plugin: bool,
	pub version: Option<String>,
}

#[tauri::command]
pub async fn pin_instance(
	state: tauri::State<'_, State>,
	instance_id: String,
	pin: bool,
) -> Result<(), String> {
	let mut data = state.data.lock().await;
	if pin {
		data.pinned.insert(instance_id);
	} else {
		data.pinned.remove(&instance_id);
	}
	fmt_err(data.write(&state.paths).context("Failed to write data"))?;

	Ok(())
}

#[tauri::command]
pub async fn get_instance_groups(
	state: tauri::State<'_, State>,
) -> Result<Vec<InstanceGroupInfo>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let groups = config
		.instance_groups
		.iter()
		.sorted_by_key(|x| x.0)
		.map(|(id, instances)| InstanceGroupInfo {
			id: id.to_string(),
			contents: instances.iter().map(ToString::to_string).collect(),
		})
		.collect();

	Ok(groups)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceGroupInfo {
	pub id: String,
	pub contents: Vec<String>,
}

#[tauri::command]
pub async fn get_instance_config(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<InstanceConfig>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(instance) = config.instances.get(&InstanceID::from(id)) else {
		return Ok(None);
	};

	Ok(Some(
		instance
			.get_config()
			.original_config_with_templates_and_plugins
			.clone(),
	))
}

#[tauri::command]
pub async fn get_editable_instance_config(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<InstanceConfig>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(instance) = config.instances.get(&InstanceID::from(id)) else {
		return Ok(None);
	};

	Ok(Some(instance.get_config().original_config.clone()))
}

#[tauri::command]
pub async fn get_template_config(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<TemplateConfig>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(template) = config.consolidated_templates.get(&TemplateID::from(id)) else {
		return Ok(None);
	};

	Ok(Some(template.clone()))
}

#[tauri::command]
pub async fn get_editable_template_config(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<TemplateConfig>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(template) = config.templates.get(&TemplateID::from(id)) else {
		return Ok(None);
	};

	Ok(Some(template.clone()))
}

#[tauri::command]
pub async fn get_base_template(state: tauri::State<'_, State>) -> Result<TemplateConfig, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	Ok(config.base_template)
}

#[tauri::command]
pub async fn write_instance_config(
	state: tauri::State<'_, State>,
	id: String,
	config: InstanceConfig,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let modifications = vec![ConfigModification::AddInstance(id.into(), config)];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &state.paths)
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn write_template_config(
	state: tauri::State<'_, State>,
	id: String,
	config: TemplateConfig,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let modifications = vec![ConfigModification::AddTemplate(id.into(), config)];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &state.paths)
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn write_base_template(
	state: tauri::State<'_, State>,
	config: TemplateConfig,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	configuration.base_template = Some(config);
	fmt_err(
		json_to_file_pretty(Config::get_path(&state.paths), &configuration)
			.context("Failed to write modified configuration"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn update_instance(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	instance_id: String,
	depth: UpdateDepth,
) -> Result<(), String> {
	update_instance_impl(&state, Arc::new(app_handle), instance_id, depth).await
}

pub async fn update_instance_impl(
	state: &State,
	app_handle: Arc<tauri::AppHandle>,
	instance_id: String,
	depth: UpdateDepth,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output_arc(app_handle));
	output.set_task("update_instance");

	let paths = state.paths.clone();
	let client = state.client.clone();
	let data = state.data.clone();
	let mut lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;
	let task = {
		let instance_id = instance_id.clone();
		let paths = paths.clone();
		async move {
			let instance_id2 = instance_id.clone();
			let Some(instance) = config.instances.get_mut(&InstanceID::from(instance_id)) else {
				bail!("Instance does not exist");
			};

			let mut ctx = InstanceUpdateContext {
				packages: &mut config.packages,
				users: &config.users,
				plugins: &config.plugins,
				prefs: &config.prefs,
				paths: &paths,
				lock: &mut lock,
				client: &client,
				output: &mut output,
			};

			instance
				.update(true, depth, &mut ctx)
				.await
				.context("Failed to update instance")?;

			let mut data_lock = data.lock().await;
			data_lock.last_resolution_errors.remove(&instance_id2);
			let _ = data_lock.write(&paths);

			Ok(())
		}
	};

	let task = tokio::spawn(unsafe { MakeSend::new(task) });
	state.register_task("update_instance_packages", task).await;

	Ok(())
}

#[tauri::command]
pub async fn update_instance_packages(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	instance_id: String,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("update_instance_packages");

	let paths = state.paths.clone();
	let client = state.client.clone();
	let data = state.data.clone();
	let mut lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;
	let task = {
		let instance_id = instance_id.clone();
		let paths = paths.clone();
		async move {
			let instance_id2 = instance_id.clone();
			let Some(instance) = config.instances.get_mut(&InstanceID::from(instance_id)) else {
				bail!("Instance does not exist");
			};

			let mut ctx = InstanceUpdateContext {
				packages: &mut config.packages,
				users: &config.users,
				plugins: &config.plugins,
				prefs: &config.prefs,
				paths: &paths,
				lock: &mut lock,
				client: &client,
				output: &mut output,
			};

			let manager = UpdateManager::new(UpdateDepth::Shallow);

			let mut core = setup_core(
				None,
				&manager.settings,
				ctx.client,
				ctx.users,
				ctx.plugins,
				ctx.paths,
				ctx.output,
			)
			.await?;

			let version = core
				.get_version(&instance.get_config().version, ctx.output)
				.await?;
			let version_info = version.get_version_info();
			let mc_version = version_info.version.clone();

			ctx.lock
				.finish(ctx.paths)
				.context("Failed to finish using lockfile")?;

			let constants = EvalConstants {
				version: mc_version.to_string(),
				loader: instance.get_config().loader.clone(),
				version_list: version_info.versions.clone(),
				language: ctx.prefs.language,
				default_stability: instance.get_config().package_stability,
			};

			nitrolaunch::instance::update::packages::update_instance_packages(
				instance, &constants, &mut ctx, false,
			)
			.await?;

			ctx.lock
				.finish(ctx.paths)
				.context("Failed to finish using lockfile")?;

			let mut data_lock = data.lock().await;
			data_lock.last_resolution_errors.remove(&instance_id2);
			let _ = data_lock.write(&paths);

			Ok(())
		}
	};

	let task = tokio::spawn(unsafe { MakeSend::new(task) });
	state.register_task("update_instance_packages", task).await;

	// Update so that there is no resolution error since we must have completed successfully

	Ok(())
}

struct MakeSend<F: Future>(Pin<Box<F>>);

unsafe impl<F: Future> Send for MakeSend<F> {}

impl<F: Future> Future for MakeSend<F> {
	type Output = F::Output;

	fn poll(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		F::poll(self.0.as_mut(), cx)
	}
}

impl<F: Future> MakeSend<F> {
	/// SAFETY: None. The future better actually be send!z
	unsafe fn new(f: F) -> Self {
		Self(Box::pin(f))
	}
}

#[tauri::command]
pub async fn get_instance_resolution_error(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<SerializableResolutionError>, String> {
	let lock = state.data.lock().await;

	Ok(lock.last_resolution_errors.get(&id).cloned())
}

#[tauri::command]
pub async fn delete_instance(state: tauri::State<'_, State>, instance: &str) -> Result<(), String> {
	fmt_err(
		delete_instance_files(instance, &state.paths)
			.await
			.context("Failed to delete instance files"),
	)?;

	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let modifications = vec![ConfigModification::RemoveInstance(instance.into())];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &state.paths)
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn delete_template(state: tauri::State<'_, State>, template: &str) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let modifications = vec![ConfigModification::RemoveTemplate(template.into())];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &state.paths)
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

/// Gets a list of the instances and templates that derive a specific template
#[tauri::command]
pub async fn get_template_users(
	state: tauri::State<'_, State>,
	template: &str,
) -> Result<Vec<(Arc<str>, InstanceOrTemplate)>, String> {
	let configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let template = template.to_string();

	let mut out = Vec::new();

	for (id, config) in &configuration.instances {
		if config.from.contains(&template) {
			out.push((id.clone(), InstanceOrTemplate::Instance));
		}
	}

	for (id, config) in &configuration.templates {
		if config.instance.from.contains(&template) {
			out.push((id.clone(), InstanceOrTemplate::Template));
		}
	}

	Ok(out)
}

#[tauri::command]
pub async fn get_last_opened_instance(
	state: tauri::State<'_, State>,
) -> Result<Option<(String, InstanceOrTemplate)>, String> {
	let data = state.data.lock().await;

	Ok(data.last_opened_instance.clone())
}

#[tauri::command]
pub async fn set_last_opened_instance(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	id: String,
	instance_or_template: InstanceOrTemplate,
) -> Result<(), String> {
	let mut data = state.data.lock().await;
	data.last_opened_instance = Some((id, instance_or_template));

	fmt_err(data.write(&state.paths))?;

	let _ = app_handle.emit("nitro_update_last_opened_instance", "");

	Ok(())
}

/// Checks if an instance has been fully updated before
#[tauri::command]
pub async fn get_instance_has_updated(
	state: tauri::State<'_, State>,
	instance: &str,
) -> Result<bool, String> {
	let lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;
	Ok(lock.has_instance_done_first_update(instance))
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum InstanceOrTemplate {
	Instance,
	Template,
}
