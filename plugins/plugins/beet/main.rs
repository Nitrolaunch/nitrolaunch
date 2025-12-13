use std::process::Command;

use anyhow::Context;
use clap::Parser;
use nitro_core::auth_crate::mc::ClientId;
use nitro_plugin::api::executable::ExecutablePlugin;
use nitro_shared::{id::InstanceID, output::NoOp};
use nitrolaunch::{config::Config, io::paths::Paths, plugin::PluginManager};

fn main() -> anyhow::Result<()> {
	let mut plugin = ExecutablePlugin::from_manifest_file("beet", include_str!("plugin.json"))?;
	plugin.subcommand(|_, arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "beet" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it =
			std::iter::once(format!("nitro {subcommand}")).chain(arg.args.into_iter().skip(1));
		let cli = Cli::parse_from(it);

		let runtime = tokio::runtime::Runtime::new()?;

		runtime.block_on(async move {
			match cli.subcommand {
				Subcommand::Link { instance, world } => link(instance, world).await,
			}
		})?;

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
	#[command(about = "List all available tools")]
	#[command(alias = "ls")]
	Link {
		/// The instance to link to
		instance: String,
		/// The world to link to
		world: String,
	},
}

async fn link(instance: String, world: String) -> anyhow::Result<()> {
	// Load the config to get the instance's game dir
	let paths = Paths::new_no_create()?;
	let plugins = PluginManager::load(&paths, &mut NoOp).await?;
	let mut config = Config::load(
		&Config::get_path(&paths),
		plugins,
		false,
		&paths,
		ClientId::new(String::new()),
		&mut NoOp,
	)
	.await?;

	let instance = config
		.instances
		.get_mut(&InstanceID::from(instance))
		.context("Instance does not exist")?;

	instance.ensure_dirs(&paths)?;
	let game_dir = instance
		.get_dirs()
		.get()
		.game_dir
		.clone()
		.context("Instance has no game dir")?;

	// Run the beet link command
	let mut command = Command::new("beet");
	command.arg("link");
	command.arg(world);

	command.env("MINECRAFT_PATH", game_dir);

	command.spawn()?.wait()?;

	Ok(())
}
