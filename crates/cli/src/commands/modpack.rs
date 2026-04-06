use anyhow::{bail, Context};
use clap::Subcommand;
use nitrolaunch::{
	config::modifications::{apply_modifications_and_write, ConfigModification},
	core::util::versions::MinecraftVersion,
	instance::{update::manager::UpdateSettings, Instance},
	pkg_crate::{PkgRequest, PkgRequestSource},
	shared::{id::InstanceID, output::NoOp, Side, UpdateDepth},
};
use reqwest::Client;

use crate::{
	commands::{call_plugin_subcommand, CmdData},
	prompt::{pick_minecraft_version, pick_side},
	secrets::get_ms_client_id,
};

#[derive(Debug, Subcommand)]
pub enum ModpackSubcommand {
	#[command(about = "Import an instance from a modpack package")]
	Import {
		/// The package of the modpack (i.e. modrinth:fabulously-optimized)
		modpack: String,
		/// The ID of the new instance
		instance: Option<String>,
		/// The Minecraft version to use
		#[arg(short, long)]
		version: Option<String>,
		/// The side of the instance. If not specified, defaults to client
		#[arg(short, long)]
		side: Option<Side>,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(command: ModpackSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match command {
		ModpackSubcommand::Import {
			modpack,
			instance,
			version,
			side,
		} => import(data, modpack, instance, version, side).await,
		ModpackSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("modpack"), data).await
		}
	}
}

async fn import(
	data: &mut CmdData<'_>,
	modpack: String,
	instance: Option<String>,
	version: Option<String>,
	side: Option<Side>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	let instance = if let Some(instance) = instance {
		instance
	} else {
		let prompt = inquire::Text::new("Enter the ID for the new instance");

		prompt.prompt()?
	};

	let instance = InstanceID::from(instance);

	if config.instances.contains_key(&instance) {
		bail!("An instance with that ID already exists");
	}

	let side = pick_side(side)?;

	let client = Client::new();

	let core = config
		.get_core(
			Some(&get_ms_client_id()),
			&UpdateSettings {
				depth: UpdateDepth::Full,
				offline_auth: false,
			},
			&client,
			&config.plugins,
			&data.paths,
			data.output,
		)
		.await?;

	let version = if let Some(version) = version {
		MinecraftVersion::Version(version.into())
	} else {
		let manifest = core
			.get_version_manifest(None, UpdateDepth::Shallow, &mut NoOp)
			.await
			.context("Failed to get version list")?;

		pick_minecraft_version(&manifest.list).await?
	};

	let version = core
		.get_version(&version, UpdateDepth::Full, data.output)
		.await
		.context("Failed to set up core version")?;

	let version_info = version.get_version_info();

	let modpack = PkgRequest::parse(modpack, PkgRequestSource::UserRequire).arc();

	let new_instance_config = Instance::create_from_modpack_package(
		&instance,
		&modpack,
		side,
		&version_info,
		&config.packages,
		&config.plugins,
		&client,
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
