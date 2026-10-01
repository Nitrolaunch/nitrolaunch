use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;
use nitro_net::download::Client;
use nitro_plugin::{api::executable::ExecutablePlugin, hook::hooks::ReplaceInstanceLaunchResult};
use nitro_shared::output::{Advanced, MessageContents, NitroOutput};
use nitrolaunch::{
	config_crate::instance::{InstanceConfig, LaunchMode, QuickPlay},
	io::paths::Paths,
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::{
	output::RemoteOutputListener,
	server::{KeyPermission, LaunchRequest},
};

mod client;
mod output;
mod server;

static BASE_TEMPLATE_ID: &str = "base_template";

fn main() -> anyhow::Result<()> {
	let mut plugin = ExecutablePlugin::from_manifest_file("remote", include_str!("plugin.json"))?;
	plugin.add_instances(|mut ctx, _| {
		let runtime = Runtime::new()?;
		let client = Client::new();
		let dir = get_dir(&ctx.get_data_dir()?);
		let plugin_config = parse_plugin_config(ctx.get_custom_config())?;

		let mut out = HashMap::new();
		for remote in plugin_config.remotes {
			let data = runtime.block_on(client::sync(&remote, &client, &dir, false));
			let data = match data {
				Ok(data) => data,
				Err(e) => {
					ctx.get_output().display(MessageContents::Error(format!(
						"Failed to sync remote {}: {e:?}",
						remote.id
					)));
					continue;
				}
			};

			out.extend(data.instances.into_iter().map(|(id, mut config)| {
				process_instance_config(&mut config, &remote.id, false);
				(process_id(&id, &remote.id).into(), config)
			}));
		}

		Ok(out)
	})?;

	plugin.add_templates(|mut ctx, _| {
		let runtime = Runtime::new()?;
		let client = Client::new();
		let dir = get_dir(&ctx.get_data_dir()?);
		let plugin_config = parse_plugin_config(ctx.get_custom_config())?;

		let mut out = HashMap::new();
		for remote in plugin_config.remotes {
			let data = runtime.block_on(client::sync(&remote, &client, &dir, false));
			let data = match data {
				Ok(data) => data,
				Err(e) => {
					ctx.get_output().display(MessageContents::Error(format!(
						"Failed to sync remote {}: {e:?}",
						remote.id
					)));
					continue;
				}
			};

			let it = std::iter::once((BASE_TEMPLATE_ID.into(), data.base_template))
				.chain(data.templates);
			out.extend(it.map(|(id, mut config)| {
				process_instance_config(&mut config.instance, &remote.id, id == BASE_TEMPLATE_ID);
				(process_id(&id, &remote.id).into(), config)
			}));
		}

		Ok(out)
	})?;

	plugin.replace_instance_launch(|mut ctx, arg| {
		if arg.config.source_plugin.is_none_or(|x| x != "remote") {
			return Ok(None);
		}
		let (remote_id, instance_id) = parse_id(&arg.id).context("Invalid remote instance ID")?;
		let plugin_config = parse_plugin_config(ctx.get_custom_config())?;
		let remote = plugin_config
			.remotes
			.into_iter()
			.find(|x| x.id == remote_id)
			.context("Remote does not exist")?;

		let runtime = Runtime::new()?;
		let client = Client::new();

		let request = LaunchRequest {
			instance: instance_id.into(),
			account: None,
			quick_play: QuickPlay::default(),
			offline: false,
		};
		let job_id = runtime
			.block_on(client::launch(&remote, &client, request))
			.context("Failed to make launch request")?;

		let mut listener = RemoteOutputListener::new(job_id, remote.clone(), client.clone());
		runtime.block_on(listener.listen(ctx.get_output()));

		Ok(Some(ReplaceInstanceLaunchResult {
			pid: None,
			stdout_path: None,
		}))
	})?;

	plugin.delete_instance(|ctx, arg| Ok(()))?;

	plugin.subcommand(|ctx, arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "remote" && subcommand != "rem" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(arg.args.into_iter().skip(1));
		let cli = Cli::parse_from(it);

		let runtime = Runtime::new()?;
		let mut o = Advanced::new();
		let plugin_config = parse_plugin_config(ctx.get_custom_config())?;

		match cli.subcommand {
			Subcommand::Start => {
				let paths = Paths::new_no_create()?;
				runtime.block_on(server::run(&paths, &mut o))?;
			}
			Subcommand::Key { subcommand } => match subcommand {
				KeySubcommand::Add { permissions } => {
					let dir = get_dir(&ctx.get_data_dir()?);
					let mut settings = server::RemoteSettings::open(&dir).unwrap_or_default();
					let key = settings.add_key(permissions);
					settings.save(&dir)?;
					o.display(MessageContents::Success(format!("Added key: {key}")));
				}
			},
		}

		Ok(())
	})?;

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
	#[command(about = "Start the remote management server")]
	Start,
	#[command(about = "Manage remote access keys")]
	Key {
		#[command(subcommand)]
		subcommand: KeySubcommand,
	},
}

#[derive(Debug, clap::Subcommand)]
enum KeySubcommand {
	#[command(about = "Add a new remote access key to the server")]
	Add {
		#[arg(long, help = "The permissions for the key")]
		permissions: Vec<KeyPermission>,
	},
}

pub fn get_dir(data_dir: &Path) -> PathBuf {
	data_dir.join("internal/remote")
}

fn parse_plugin_config(config: Option<&str>) -> anyhow::Result<PluginConfig> {
	if let Some(config) = config {
		serde_json::from_str(config).context("Failed to parse config")
	} else {
		Ok(PluginConfig::default())
	}
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
struct PluginConfig {
	remotes: Vec<client::RemoteSettings>,
}

fn process_instance_config(config: &mut InstanceConfig, remote_id: &str, is_base_template: bool) {
	config.source_plugin = Some("remote".into());
	config
		.from
		.iter_mut()
		.for_each(|x| *x = process_id(x, remote_id));
	if !is_base_template {
		config
			.from
			.push_front(process_id(BASE_TEMPLATE_ID, remote_id));
	}
	config.dir = Some("none".into());
	config.launch_mode = LaunchMode::Wait;
}

fn process_id(id: &str, remote_id: &str) -> String {
	format!("{remote_id}:{id}")
}

fn parse_id(id: &str) -> Option<(&str, &str)> {
	id.split_once(':')
}
