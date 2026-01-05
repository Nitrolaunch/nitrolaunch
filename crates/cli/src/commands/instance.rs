use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context};
use clap::Subcommand;
use color_print::{cprint, cprintln};
use inquire::{Confirm, MultiSelect, Select};
use itertools::Itertools;
use nitrolaunch::config::modifications::{apply_modifications_and_write, ConfigModification};
use nitrolaunch::config::Config;
use nitrolaunch::config_crate::instance::InstanceConfig;
use nitrolaunch::instance::transfer::load_formats;
use nitrolaunch::instance::update::InstanceUpdateContext;
use nitrolaunch::instance::Instance;
use nitrolaunch::io::lock::Lockfile;
use nitrolaunch::plugin_crate::hook::hooks::{DeleteInstance, SaveInstanceConfigArg};
use nitrolaunch::shared::id::InstanceID;
use nitrolaunch::shared::output::{MessageContents, MessageLevel};
use nitrolaunch::shared::util::to_string_json;
use nitrolaunch::shared::versions::{MinecraftLatestVersion, MinecraftVersionDeser};

use nitrolaunch::instance::launch::LaunchSettings;
use nitrolaunch::shared::lang::translate::TranslationKey;
use nitrolaunch::shared::loaders::Loader;
use nitrolaunch::shared::{output::NitroOutput, Side, UpdateDepth};
use reqwest::Client;

use super::CmdData;
use crate::commands::call_plugin_subcommand;
use crate::commands::config::edit_temp_file;
use crate::output::{icons_enabled, HYPHEN_POINT, INSTANCE, LOADER, PACKAGE, VERSION};
use crate::secrets::get_ms_client_id;

#[derive(Debug, Subcommand)]
pub enum InstanceSubcommand {
	#[command(about = "List all instances")]
	#[clap(alias = "ls")]
	List {
		/// Whether to remove formatting and warnings from the output
		#[arg(short, long)]
		raw: bool,
		/// Filter by instance side
		#[arg(short, long)]
		side: Option<Side>,
	},
	#[command(about = "Print useful information about an instance")]
	Info { instance: Option<String> },
	#[command(about = "Launch instances to play the game")]
	Launch {
		/// An optional user to choose when launching
		#[arg(short, long)]
		user: Option<String>,
		/// Whether to launch in offline mode, skipping authentication. This only works
		/// if you have authenticated at least once
		#[arg(short, long)]
		offline: bool,
		/// Whether to skip updating on the first launch. Can cause problems!
		#[arg(long)]
		skip_update: bool,
		/// The instance to launch
		instance: Option<String>,
	},
	#[command(about = "Update versions, files, and packages of an instance")]
	Update {
		/// Whether to force update files that have already been downloaded
		#[arg(short, long)]
		force: bool,
		/// Whether to update all instances
		#[arg(short, long)]
		all: bool,
		/// Whether to skip updating packages
		#[arg(short = 'P', long)]
		skip_packages: bool,
		/// Additional instance groups to update
		#[arg(short, long)]
		groups: Vec<String>,
		/// The instances to update
		instances: Vec<String>,
	},
	#[command(about = "Easily create a new instance")]
	Add,
	#[command(about = "Delete an instance and its files forever")]
	Delete {
		/// The instance to delete
		instance: Option<String>,
	},
	#[command(about = "Edit configuration for an instance")]
	Edit {
		/// The instance to edit
		instance: Option<String>,
	},
	#[command(about = "Import an instance from another launcher")]
	Import {
		/// The path to the instance
		path: String,
		/// The ID of the new instance
		instance: String,
		/// Which format to use
		#[arg(short, long)]
		format: Option<String>,
	},
	#[command(about = "Export an instance for use in another launcher")]
	Export {
		/// The ID of the instance to export
		instance: Option<String>,
		/// Which format to use
		#[arg(short, long)]
		format: Option<String>,
		/// Where to export the instance to. Defaults to ./<instance-id>.zip
		#[arg(short, long)]
		output: Option<String>,
	},
	#[command(about = "Print the directory of an instance")]
	Dir {
		/// The instance to print the directory of
		instance: Option<String>,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(command: InstanceSubcommand, mut data: CmdData<'_>) -> anyhow::Result<()> {
	match command {
		InstanceSubcommand::List { raw, side } => list(&mut data, raw, side).await,
		InstanceSubcommand::Launch {
			user,
			offline,
			skip_update,
			instance,
		} => launch(instance, user, offline, skip_update, data).await,
		InstanceSubcommand::Info { instance } => info(&mut data, instance).await,
		InstanceSubcommand::Update {
			force,
			all,
			skip_packages,
			groups,
			instances,
		} => update(&mut data, instances, groups, all, force, skip_packages).await,
		InstanceSubcommand::Dir { instance } => dir(&mut data, instance).await,
		InstanceSubcommand::Add => add(&mut data).await,
		InstanceSubcommand::Import {
			instance,
			path,
			format,
		} => import(&mut data, instance, path, format).await,
		InstanceSubcommand::Export {
			instance,
			format,
			output,
		} => export(&mut data, instance, format, output).await,
		InstanceSubcommand::Delete { instance } => delete(&mut data, instance).await,
		InstanceSubcommand::Edit { instance } => edit(&mut data, instance).await,
		InstanceSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("instance"), &mut data).await
		}
	}
}

async fn list(data: &mut CmdData<'_>, raw: bool, side: Option<Side>) -> anyhow::Result<()> {
	data.ensure_config(!raw).await?;
	let config = data.config.get_mut();

	for (id, instance) in config.instances.iter().sorted_by_key(|x| x.0) {
		if let Some(side) = side {
			if instance.get_side() != side {
				continue;
			}
		}

		if raw {
			println!("{id}");
		} else {
			match instance.get_side() {
				Side::Client => cprintln!("{}<g>{}", HYPHEN_POINT, id),
				Side::Server => cprintln!("{}<b>{}", HYPHEN_POINT, id),
			}
		}
	}

	Ok(())
}

async fn info(data: &mut CmdData<'_>, id: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let id = pick_instance(id, config)?;

	fn print_indent() {
		print!("   ");
	}

	let instance = config
		.instances
		.get(&id)
		.with_context(|| format!("Unknown instance '{id}'"))?;

	if icons_enabled() {
		print!("{} ", INSTANCE);
	}
	cprintln!("<s><g>Instance <b>{}", id);
	print_indent();
	if icons_enabled() {
		print!("{} ", VERSION);
	}
	cprintln!("<s>Version:</s> <g>{}", instance.get_config().version);

	print_indent();
	cprint!("{}Type: ", HYPHEN_POINT);
	match instance.get_side() {
		Side::Client => cprint!("<y!>Client"),
		Side::Server => cprint!("<c!>Server"),
	}
	cprintln!();

	print_indent();
	if icons_enabled() {
		print!("{} ", LOADER);
	}
	cprintln!("<s>Loader:</s> <g>{}", instance.get_config().loader);

	print_indent();
	if icons_enabled() {
		print!("{} ", PACKAGE);
	}
	cprintln!("<s>Packages:");
	for pkg in instance.get_configured_packages() {
		print_indent();
		cprint!("{}", HYPHEN_POINT);
		cprint!("<b!>{}<g!>", pkg.id);
		cprintln!();
	}

	Ok(())
}

pub async fn launch(
	instance: Option<String>,
	user: Option<String>,
	offline: bool,
	skip_update: bool,
	mut data: CmdData<'_>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let instance_id = pick_instance(instance, config).context("Failed to pick instance")?;

	let instance = config
		.instances
		.get_mut(&instance_id)
		.context("Instance does not exist")?;

	// Perform first update if needed
	if !skip_update {
		let mut lock = Lockfile::open(&data.paths).context("Failed to open lockfile")?;
		if !lock.has_instance_done_first_update(&instance_id) {
			cprintln!("<s>Performing first update of instance...");

			let client = Client::new();
			let mut ctx = InstanceUpdateContext {
				packages: &mut config.packages,
				users: &config.users,
				plugins: &config.plugins,
				prefs: &config.prefs,
				paths: &data.paths,
				lock: &mut lock,
				client: &client,
				output: data.output,
			};

			instance
				.update(true, UpdateDepth::Full, &mut ctx)
				.await
				.context("Failed to perform first update for instance")?;
		}
	}

	if let Some(user) = user {
		config
			.users
			.choose_user(&user)
			.context("Failed to choose user")?;
	}

	let launch_settings = LaunchSettings {
		ms_client_id: get_ms_client_id(),
		offline_auth: offline,
		pipe_stdin: true,
	};
	let instance_handle = instance
		.launch(
			&data.paths,
			&mut config.users,
			&config.plugins,
			launch_settings,
			data.output,
		)
		.await
		.context("Instance failed to launch")?;

	// Drop the config early so that it isn't wasting memory while the instance is running
	let plugins = config.plugins.clone();
	std::mem::drop(data.config);
	// Unload plugins that we don't need anymore

	instance_handle
		.wait(&plugins, &data.paths, data.output)
		.await
		.context("Failed to wait for instance child process")?;

	Ok(())
}

async fn dir(data: &mut CmdData<'_>, instance: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;

	let instance = pick_instance(instance, data.config.get()).context("Failed to pick instance")?;
	let instance = data
		.config
		.get_mut()
		.instances
		.get_mut(&instance)
		.context("Instance does not exist")?;
	instance.ensure_dirs(&data.paths)?;

	if let Some(game_dir) = &instance.get_dirs().get().game_dir {
		println!("{game_dir:?}");
	} else {
		bail!("Instance has no game dir");
	}

	Ok(())
}

async fn update(
	data: &mut CmdData<'_>,
	instances: Vec<String>,
	groups: Vec<String>,
	all: bool,
	force: bool,
	skip_packages: bool,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let mut ids: Vec<InstanceID> = if all {
		config.instances.keys().cloned().collect()
	} else {
		instances.into_iter().map(InstanceID::from).collect()
	};

	for group in groups {
		let group = Arc::from(group);
		let group = config
			.instance_groups
			.get(&group)
			.with_context(|| format!("Instance group '{group}' does not exist"))?;
		ids.extend(group.clone());
	}

	if ids.is_empty() {
		ids = pick_instances(config)?;
	}

	let client = Client::new();
	let mut lock = Lockfile::open(&data.paths).context("Failed to open lockfile")?;
	for id in ids {
		let instance = config
			.instances
			.get_mut(&id)
			.with_context(|| format!("Unknown instance '{id}'"))?;

		let mut ctx = InstanceUpdateContext {
			packages: &mut config.packages,
			users: &config.users,
			plugins: &config.plugins,
			prefs: &config.prefs,
			paths: &data.paths,
			lock: &mut lock,
			client: &client,
			output: data.output,
		};

		let depth = if force {
			UpdateDepth::Force
		} else {
			UpdateDepth::Full
		};

		instance
			.update(!skip_packages, depth, &mut ctx)
			.await
			.context("Failed to update instance")?;

		// Clear the package registry to prevent dependency chains in requests being carried over
		config.packages.clear();

		// Mark the instance as having completed its first update
		lock.update_instance_has_done_first_update(instance.get_id());
		lock.finish(&data.paths)
			.context("Failed to finish using lockfile")?;
	}

	Ok(())
}

async fn add(data: &mut CmdData<'_>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut config = data.get_raw_config()?;

	// Build the instance
	let id = inquire::Text::new("What is the ID for the instance?").prompt()?;
	let id = InstanceID::from(id);
	let version = inquire::Text::new("What Minecraft version should the instance be?").prompt()?;
	let version = match version.as_str() {
		"latest" => MinecraftVersionDeser::Latest(MinecraftLatestVersion::Release),
		"latest_snapshot" => MinecraftVersionDeser::Latest(MinecraftLatestVersion::Snapshot),
		other => MinecraftVersionDeser::Version(other.into()),
	};
	let side_options = vec![Side::Client, Side::Server];
	let side =
		inquire::Select::new("What side should the instance be on?", side_options).prompt()?;

	let loader_options = match side {
		Side::Client => {
			vec![Loader::Vanilla, Loader::Fabric, Loader::Quilt]
		}
		Side::Server => {
			vec![
				Loader::Vanilla,
				Loader::Fabric,
				Loader::Quilt,
				Loader::Paper,
				Loader::Sponge,
				Loader::Folia,
			]
		}
	};
	let loader =
		inquire::Select::new("What loader should the instance use?", loader_options).prompt()?;

	let instance_config = InstanceConfig {
		side: Some(side),
		version: Some(version),
		loader: Some(to_string_json(&loader)),
		..Default::default()
	};

	apply_modifications_and_write(
		&mut config,
		vec![ConfigModification::AddInstance(id, instance_config)],
		&data.paths,
		&data.config.get().plugins,
		data.output,
	)
	.await
	.context("Failed to write modified config")?;

	data.output.display(
		MessageContents::Success("Instance added".into()),
		MessageLevel::Important,
	);

	Ok(())
}

async fn import(
	data: &mut CmdData<'_>,
	instance: String,
	path: String,
	format: Option<String>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	let instance = InstanceID::from(instance);

	if config.instances.contains_key(&instance) {
		bail!("An instance with that ID already exists");
	}

	// Figure out the format
	let formats = load_formats(&config.plugins, &data.paths, data.output)
		.await
		.context("Failed to get available transfer formats")?;
	let format = if let Some(format) = &format {
		format
	} else {
		let options: Vec<_> = formats.iter_format_names().collect();
		if options.is_empty() {
			bail!(
				"{}",
				data.output.translate(TranslationKey::NoTransferFormats)
			);
		}
		inquire::Select::new("What format is the imported instance in?", options).prompt()?
	};

	let new_instance_config = Instance::import(
		&instance,
		format,
		&PathBuf::from(path),
		&formats,
		&config.plugins,
		&data.paths,
		data.output,
	)
	.await
	.context("Failed to import the new instance")?;

	let mut config2 = data.get_raw_config()?;
	apply_modifications_and_write(
		&mut config2,
		vec![ConfigModification::AddInstance(
			instance,
			new_instance_config,
		)],
		&data.paths,
		&config.plugins,
		data.output,
	)
	.await
	.context("Failed to write modified config")?;

	Ok(())
}

async fn export(
	data: &mut CmdData<'_>,
	instance: Option<String>,
	format: Option<String>,
	output: Option<String>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let instance = pick_instance(instance, config)?;

	// Figure out the format
	let formats = load_formats(&config.plugins, &data.paths, data.output)
		.await
		.context("Failed to get available transfer formats")?;
	let format = if let Some(format) = &format {
		format
	} else {
		let options: Vec<_> = formats.iter_format_names().collect();
		if options.is_empty() {
			bail!(
				"{}",
				data.output.translate(TranslationKey::NoTransferFormats)
			);
		}
		inquire::Select::new("What format is the exported instance in?", options).prompt()?
	};

	let result_path = if let Some(output) = output {
		PathBuf::from(output)
	} else {
		let current_dir = std::env::current_dir()?;
		current_dir.join(format!("{instance}.zip"))
	};

	let instance = config
		.instances
		.get_mut(&instance)
		.context("The provided instance does not exist")?;

	let lock = Lockfile::open(&data.paths).context("Failed to open Lockfile")?;

	instance
		.export(
			format,
			&result_path,
			&formats,
			&config.plugins,
			&lock,
			&data.paths,
			data.output,
		)
		.await
		.context("Failed to export instance")?;

	Ok(())
}

async fn delete(data: &mut CmdData<'_>, id: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut raw_config = data.get_raw_config()?;
	let config = data.config.get_mut();

	let id = pick_instance(id, config)?;

	let instance = config
		.instances
		.get_mut(&id)
		.with_context(|| format!("Unknown instance '{id}'"))?;

	let prompt = Confirm::new(
		"Are you SURE you want to delete this instance? This will remove world saves as well! (y/n)",
	);
	if !prompt.prompt()? {
		cprintln!("<r>Cancelled.");
		return Ok(());
	}

	let mut process = data.output.get_process();
	process.display(
		MessageContents::StartProcess("Deleting instance".into()),
		MessageLevel::Important,
	);

	instance
		.delete_files(&data.paths)
		.await
		.context("Failed to delete instance")?;

	if let Some(source_plugin) = &instance.get_config().original_config.source_plugin {
		if !instance.get_config().original_config.is_deletable {
			bail!("Plugin instance does not support deletion");
		}

		let arg = SaveInstanceConfigArg {
			id: id.to_string(),
			config: instance.get_config().original_config.clone(),
		};

		let result = config
			.plugins
			.call_hook_on_plugin(
				DeleteInstance,
				source_plugin,
				&arg,
				&data.paths,
				process.deref_mut(),
			)
			.await?;
		if let Some(result) = result {
			result.result(process.deref_mut()).await?;
		}
	} else {
		let modifications = vec![ConfigModification::RemoveInstance(id.into())];
		apply_modifications_and_write(
			&mut raw_config,
			modifications,
			&data.paths,
			&config.plugins,
			process.deref_mut(),
		)
		.await
		.context("Failed to modify and write config")?;
	}

	process.display(
		MessageContents::Success("Instance deleted".into()),
		MessageLevel::Important,
	);

	Ok(())
}

async fn edit(data: &mut CmdData<'_>, id: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut raw_config = data.get_raw_config()?;
	let config = data.config.get_mut();

	let id = pick_instance(id, config)?;

	let instance = config
		.instances
		.get_mut(&id)
		.with_context(|| format!("Unknown instance '{id}'"))?;

	let mut inst_config = instance.get_config().original_config.clone();
	if inst_config.source_plugin.is_some() && !inst_config.is_editable {
		bail!("This plugin instance does not support editing");
	}
	inst_config.remove_plugin_only_fields();

	let text = serde_json::to_string_pretty(&inst_config).context("Failed to serialize config")?;
	let edited = edit_temp_file(&text, &format!("Editing instance {id}"), &data.paths)?;
	let mut new_config: InstanceConfig = serde_json::from_str(&edited)
		.context("Failed to serialize. Make sure your config is valid JSON")?;
	new_config.restore_plugin_only_fields(&inst_config);

	let modifications = vec![ConfigModification::AddInstance(id.into(), new_config)];
	apply_modifications_and_write(
		&mut raw_config,
		modifications,
		&data.paths,
		&config.plugins,
		data.output,
	)
	.await
	.context("Failed to modify and write config")?;

	data.output.display(
		MessageContents::Success("Changes saved".into()),
		MessageLevel::Important,
	);

	Ok(())
}

/// Pick which instance to use
pub fn pick_instance(instance: Option<String>, config: &Config) -> anyhow::Result<InstanceID> {
	if let Some(instance) = instance {
		Ok(instance.into())
	} else {
		let options = config.instances.keys().sorted().collect();
		let selection = Select::new("Choose an instance", options)
			.prompt()
			.context("Prompt failed")?;

		Ok(selection.to_owned())
	}
}

/// Pick which instances to use
pub fn pick_instances(config: &Config) -> anyhow::Result<Vec<InstanceID>> {
	let options = config.instances.keys().sorted().cloned().collect();
	let selection = MultiSelect::new("Choose instances", options)
		.prompt()
		.context("Prompt failed")?;

	Ok(selection)
}
