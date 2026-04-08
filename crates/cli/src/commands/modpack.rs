use std::ops::DerefMut;

use anyhow::{bail, Context};
use clap::Subcommand;
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::instance::InstanceConfig,
	core::util::versions::MinecraftVersion,
	instance::{update::manager::UpdateSettings, Instance},
	io::paths::Paths,
	pkg_crate::{PkgRequest, PkgRequestSource},
	shared::{
		id::InstanceID,
		output::{MessageContents, NitroOutput, NoOp},
		pkg::ArcPkgReq,
		Side, UpdateDepth,
	},
};
use reqwest::Client;

use crate::{
	commands::{call_plugin_subcommand, CmdData},
	prompt::{pick_instance_id, pick_minecraft_version, pick_side},
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
		/// The Minecraft version to use
		#[arg(short, long)]
		version: Option<String>,
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
			version,
			side,
		} => install(data, modpack, instance, version, side).await,
		ModpackSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("modpack"), data).await
		}
	}
}

async fn install(
	data: &mut CmdData<'_>,
	modpack: String,
	instance: Option<String>,
	version: Option<String>,
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
		version,
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
	version: Option<String>,
	side: Option<Side>,
	config: &Config,
	paths: &Paths,
	o: &mut impl NitroOutput,
) -> anyhow::Result<InstanceConfig> {
	let client = Client::new();

	let mut process = o.get_process();
	process.display(MessageContents::StartProcess("Fetching modpack".into()));

	let modpack = config
		.packages
		.get(&req, &paths, &client, process.deref_mut())
		.await?;

	let props = modpack.get_properties(&paths, &client).await?;

	process.display(MessageContents::Success("Modpack fetched".into()));
	process.finish();

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

	// Figure out version from the ones available from the modpack
	let version = if let Some(version) = version {
		if let Some(versions) = &props.supported_versions {
			let manifest = core
				.get_version_manifest(None, UpdateDepth::Full, &mut NoOp)
				.await
				.context("Failed to get version list")?;

			if !versions
				.iter()
				.any(|x| x.matches_single(&version, &manifest.list))
			{
				bail!("Selected Minecraft version is not supported by the modpack");
			}
		}
		MinecraftVersion::Version(version.into())
	} else {
		let manifest = core
			.get_version_manifest(None, UpdateDepth::Full, &mut NoOp)
			.await
			.context("Failed to get version list")?;

		let available = if let Some(versions) = &props.supported_versions {
			versions
				.iter()
				.flat_map(|x| x.get_matches(&manifest.list))
				.collect()
		} else {
			manifest.list.clone()
		};

		pick_minecraft_version(&available).await?
	};

	let version = core
		.get_version(&version, UpdateDepth::Full, o)
		.await
		.context("Failed to set up core version")?;

	let version_info = version.get_version_info();

	Instance::create_from_modpack_package(
		&instance,
		&req,
		side,
		&version_info,
		&config.packages,
		&config.plugins,
		&client,
		&paths,
		o,
	)
	.await
	.context("Failed to import the new instance")
}
