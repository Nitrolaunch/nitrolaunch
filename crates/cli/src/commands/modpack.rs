use anyhow::{bail, Context};
use clap::Subcommand;
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::instance::InstanceConfig,
	instance::{update::manager::UpdateSettings, Instance},
	io::paths::Paths,
	pkg_crate::{PkgRequest, PkgRequestSource},
	shared::{id::InstanceID, output::NitroOutput, pkg::ArcPkgReq, Side, UpdateDepth},
};
use reqwest::Client;

use crate::{
	commands::{call_plugin_subcommand, CmdData},
	prompt::{pick_instance_id, pick_side},
	secrets::get_ms_client_id,
};

#[derive(Debug, Subcommand)]
pub enum ModpackSubcommand {
	#[command(about = "Create an instance from a modpack package")]
	Install {
		/// The package of the modpack (i.e. modrinth:fabulously-optimized)
		modpack: String,
		/// The ID of the new instance
		instance: Option<String>,
		/// The side of the instance
		#[arg(short, long)]
		side: Option<Side>,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(command: ModpackSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match command {
		ModpackSubcommand::Install {
			modpack,
			instance,
			side,
		} => install(data, modpack, instance, side).await,
		ModpackSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("modpack"), data).await
		}
	}
}

async fn install(
	data: &mut CmdData<'_>,
	modpack: String,
	instance: Option<String>,
	side: Option<Side>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	let req = PkgRequest::parse(modpack, PkgRequestSource::UserRequire).arc();

	let instance = if let Some(instance) = instance {
		InstanceID::from(instance)
	} else {
		pick_instance_id()?
	};

	let new_instance_config = install_into_config(
		&req,
		instance.clone(),
		side,
		config,
		&data.paths,
		data.output,
	)
	.await?;

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

/// Downloads and installs a modpack into a new instance folder, returning the config for the instance
pub async fn install_into_config(
	req: &ArcPkgReq,
	instance: InstanceID,
	side: Option<Side>,
	config: &Config,
	paths: &Paths,
	o: &mut impl NitroOutput,
) -> anyhow::Result<InstanceConfig> {
	let client = Client::new();

	if config.instances.contains_key(&instance) {
		bail!("An instance with that ID already exists");
	}

	let side = pick_side(side)?;

	let core = config
		.get_core(
			Some(&get_ms_client_id()),
			&UpdateSettings {
				depth: UpdateDepth::Shallow,
				offline_auth: false,
			},
			&client,
			&config.plugins,
			&paths,
			o,
		)
		.await?;

	let version_manifest = core
		.get_version_manifest(None, UpdateDepth::Full, o)
		.await?;

	Instance::create_from_modpack_package(
		&instance,
		&req,
		side,
		version_manifest.list.clone(),
		&config.packages,
		&config.plugins,
		&client,
		&paths,
		o,
	)
	.await
	.context("Failed to import the new instance")
}
