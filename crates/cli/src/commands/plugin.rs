use anyhow::{bail, Context};
use clap::Subcommand;
use color_print::cprintln;
use itertools::Itertools;
use nitrolaunch::core::io::json_from_file;
use nitrolaunch::plugin::install::get_verified_plugins;
use nitrolaunch::plugin::PluginManager;
use nitrolaunch::plugin_crate::plugin::PluginManifest;
use nitrolaunch::shared::lang::translate::TranslationKey;
use nitrolaunch::shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitrolaunch::shared::translate;
use nitrolaunch::shared::versions::parse_single_versioned_string;
use reqwest::Client;
use std::ops::DerefMut;

use super::CmdData;
use crate::commands::call_plugin_subcommand;
use crate::output::{CHECK, HYPHEN_POINT};

#[derive(Debug, Subcommand)]
pub enum PluginSubcommand {
	#[command(about = "List all enabled plugins")]
	#[clap(alias = "ls")]
	List {
		/// Whether to remove formatting and warnings from the output
		#[arg(short, long)]
		raw: bool,
		/// Whether to filter only the loaded plugins
		#[arg(short, long)]
		loaded: bool,
	},
	#[command(about = "Print useful information about a plugin")]
	Info { plugin: String },
	#[command(about = "Install one or more plugins from the verified list")]
	Install {
		/// The plugins to install
		plugins: Vec<String>,
		/// The version of the plugins to install. Defaults to the latest version.
		/// You can also specify each plugin name with @version at the end to override
		/// this global version per-plugin
		#[arg(short, long)]
		version: Option<String>,
	},
	#[command(about = "Uninstall a plugin")]
	Uninstall { plugin: String },
	#[command(about = "Browse installable plugins")]
	Browse,
	#[command(about = "Enable a plugin")]
	Enable { plugin: String },
	#[command(about = "Disable a plugin")]
	Disable { plugin: String },
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(command: PluginSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match command {
		PluginSubcommand::List { raw, loaded } => list(data, raw, loaded).await,
		PluginSubcommand::Info { plugin } => info(data, plugin).await,
		PluginSubcommand::Install { plugins, version } => install(data, plugins, version).await,
		PluginSubcommand::Uninstall { plugin } => uninstall(data, plugin).await,
		PluginSubcommand::Browse => browse(data).await,
		PluginSubcommand::Enable { plugin } => enable(data, plugin).await,
		PluginSubcommand::Disable { plugin } => disable(data, plugin).await,
		PluginSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("plugin"), data).await
		}
	}
}

async fn list(data: &mut CmdData<'_>, raw: bool, loaded: bool) -> anyhow::Result<()> {
	data.ensure_config(!raw).await?;
	let config = data.config.get_mut();

	let mut available_plugins = PluginManager::get_available_plugins(&data.paths)
		.context("Failed to get list of available plugins")?;
	available_plugins.sort_by_key(|x| x.0.clone());

	let lock = config.plugins.get_lock().await;
	let loaded_plugins: Vec<_> = lock.manager.iter_plugins().map(|x| x.get_id()).collect();

	for (plugin_id, plugin_path) in available_plugins {
		let is_loaded = loaded_plugins.contains(&&plugin_id);
		if loaded && !is_loaded {
			continue;
		}

		if raw {
			println!("{}", plugin_id);
		} else if is_loaded {
			cprintln!(
				"{}[<s><g>{CHECK}</>] {}</> [<g>Enabled</>]",
				HYPHEN_POINT,
				plugin_id
			);
		} else {
			let is_valid = json_from_file::<PluginManifest>(plugin_path).is_ok();
			if is_valid {
				cprintln!("{}[ ] {} [Disabled]", HYPHEN_POINT, plugin_id);
			} else {
				cprintln!("{}[ ] <r>{} [Invalid]", HYPHEN_POINT, plugin_id);
			}
		}
	}

	Ok(())
}

async fn info(data: &mut CmdData<'_>, plugin: String) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let lock = config.plugins.get_lock().await;
	let plugin = lock
		.manager
		.iter_plugins()
		.find(|x| x.get_id() == &plugin)
		.context("Plugin does not exist")?;

	cprintln!(
		"<s>Plugin <b>{}</>:",
		plugin
			.get_manifest()
			.meta
			.name
			.as_ref()
			.unwrap_or(plugin.get_id())
	);
	if let Some(description) = &plugin.get_manifest().meta.description {
		cprintln!("{}", description);
	}
	cprintln!("{}<s>ID:</> {}", HYPHEN_POINT, plugin.get_id());

	Ok(())
}

pub(crate) async fn install(
	data: &mut CmdData<'_>,
	plugins: Vec<String>,
	version: Option<String>,
) -> anyhow::Result<()> {
	if plugins.is_empty() {
		bail!("No plugins were provided to install");
	}

	let client = Client::new();

	let verified_list = get_verified_plugins(&client, false)
		.await
		.context("Failed to get verified plugin list")?;

	if plugins.len() > 1 && version.is_some() {
		bail!("Cannot specify a version for multiple plugins");
	}

	for plugin in plugins {
		let (plugin_id, version_override) = parse_single_versioned_string(&plugin);

		let version = version_override.or(version.as_deref());

		let Some(plugin) = verified_list.get(plugin_id) else {
			bail!("Unknown plugin '{plugin_id}'");
		};

		let mut process = data.output.get_process();

		let message = translate!(process, StartInstallingPlugin, "plugin" = plugin_id);
		process.display(
			MessageContents::StartProcess(message),
			MessageLevel::Important,
		);
		plugin
			.install(version, &data.paths, &client, process.deref_mut())
			.await
			.context("Failed to install plugin")?;

		let message = process
			.translate(TranslationKey::FinishInstallingPlugin)
			.to_string();
		process.display(MessageContents::Success(message), MessageLevel::Important);
	}

	Ok(())
}

async fn uninstall(data: &mut CmdData<'_>, plugin: String) -> anyhow::Result<()> {
	let Ok(result) = data.output.prompt_yes_no(
		false,
		MessageContents::Simple("Are you sure you want to delete this plugin?".into()),
	) else {
		return Ok(());
	};
	if !result {
		cprintln!("Keeping plugin");
		return Ok(());
	}

	PluginManager::uninstall_plugin(&plugin, &data.paths).context("Failed to remove plugin")?;

	data.output.display(
		MessageContents::Success("Plugin removed".into()),
		MessageLevel::Important,
	);

	Ok(())
}

async fn browse(data: &mut CmdData<'_>) -> anyhow::Result<()> {
	let _ = data;

	let client = Client::new();

	let verified_list = get_verified_plugins(&client, false)
		.await
		.context("Failed to get verified plugin list")?;

	data.output.display(
		MessageContents::Header("Available plugins:".into()),
		MessageLevel::Important,
	);
	for plugin in verified_list
		.values()
		.sorted_by_cached_key(|x| x.id.clone())
	{
		if let Some(description) = &plugin.meta.description {
			cprintln!("{}<s>{}</> - {}", HYPHEN_POINT, plugin.id, description);
		} else {
			cprintln!("{}<s>{}</>", HYPHEN_POINT, plugin.id);
		}
	}

	Ok(())
}

async fn enable(data: &mut CmdData<'_>, plugin: String) -> anyhow::Result<()> {
	PluginManager::enable_plugin(&plugin, &data.paths)?;

	data.output.display(
		MessageContents::Success("Plugin enabled".into()),
		MessageLevel::Important,
	);

	Ok(())
}

async fn disable(data: &mut CmdData<'_>, plugin: String) -> anyhow::Result<()> {
	PluginManager::disable_plugin(&plugin, &data.paths)?;

	data.output.display(
		MessageContents::Success("Plugin disabled".into()),
		MessageLevel::Important,
	);

	Ok(())
}
