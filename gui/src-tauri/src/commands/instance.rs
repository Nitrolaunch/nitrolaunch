use crate::data::InstanceIcon;
use crate::output::{LauncherOutput, SerializableResolutionError};
use crate::State;
use anyhow::{bail, Context};
use itertools::Itertools;
use mcvm::config::modifications::{apply_modifications_and_write, ConfigModification};
use mcvm::config::Config;
use mcvm::config_crate::instance::InstanceConfig;
use mcvm::config_crate::profile::ProfileConfig;
use mcvm::core::io::json_to_file_pretty;
use mcvm::instance::update::InstanceUpdateContext;
use mcvm::io::lock::Lockfile;
use mcvm::shared::id::{InstanceID, ProfileID};
use mcvm::shared::output::NoOp;
use mcvm::shared::{Side, UpdateDepth};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

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
				icon: data.instance_icons.get(&id).cloned(),
				pinned: data.pinned.contains(&id),
				id,
				name: instance.get_config().name.clone(),
				side: Some(instance.get_side()),
			}
		})
		.collect();

	Ok(instances)
}

#[tauri::command]
pub async fn get_profiles(state: tauri::State<'_, State>) -> Result<Vec<InstanceInfo>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let data = state.data.lock().await;

	let profiles = config
		.profiles
		.iter()
		.sorted_by_key(|x| x.0)
		.map(|(id, profile)| {
			let id = id.to_string();
			InstanceInfo {
				icon: data.profile_icons.get(&id).cloned(),
				pinned: false,
				id,
				name: None,
				side: profile.instance.side,
			}
		})
		.collect();

	Ok(profiles)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceInfo {
	pub id: String,
	pub name: Option<String>,
	pub side: Option<Side>,
	pub icon: Option<InstanceIcon>,
	pub pinned: bool,
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
			.original_config_with_profiles_and_plugins
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
pub async fn get_profile_config(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<ProfileConfig>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(profile) = config.consolidated_profiles.get(&ProfileID::from(id)) else {
		return Ok(None);
	};

	Ok(Some(profile.clone()))
}

#[tauri::command]
pub async fn get_editable_profile_config(
	state: tauri::State<'_, State>,
	id: String,
) -> Result<Option<ProfileConfig>, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let Some(profile) = config.profiles.get(&ProfileID::from(id)) else {
		return Ok(None);
	};

	Ok(Some(profile.clone()))
}

#[tauri::command]
pub async fn get_global_profile(state: tauri::State<'_, State>) -> Result<ProfileConfig, String> {
	let config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	Ok(config.global_profile)
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
pub async fn write_profile_config(
	state: tauri::State<'_, State>,
	id: String,
	config: ProfileConfig,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	let modifications = vec![ConfigModification::AddProfile(id.into(), config)];
	fmt_err(
		apply_modifications_and_write(&mut configuration, modifications, &state.paths)
			.context("Failed to modify and write config"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn write_global_profile(
	state: tauri::State<'_, State>,
	config: ProfileConfig,
) -> Result<(), String> {
	let mut configuration =
		fmt_err(Config::open(&Config::get_path(&state.paths)).context("Failed to load config"))?;

	configuration.global_profile = Some(config);
	fmt_err(
		json_to_file_pretty(&Config::get_path(&state.paths), &configuration)
			.context("Failed to write modified configuration"),
	)?;

	Ok(())
}

#[tauri::command]
pub async fn update_instance(
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
	output.set_task("update_instance");

	let paths = state.paths.clone();
	let client = state.client.clone();
	let mut lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;
	let task = {
		let instance_id = instance_id.clone();
		let paths = paths.clone();
		async move {
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
				.update(true, UpdateDepth::Full, &mut ctx)
				.await
				.context("Failed to update instance")
		}
	};
	fmt_err(fmt_err(tokio::spawn(unsafe { MakeSend::new(task) }).await)?)?;

	// Update so that there is no resolution error since we must have completed successfully
	let mut lock = state.data.lock().await;
	lock.last_resolution_errors.remove(&instance_id);
	let _ = lock.write(&paths);

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

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum InstanceOrProfile {
	Instance,
	Profile,
}
