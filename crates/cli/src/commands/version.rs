use clap::Subcommand;
use color_print::{cprint, cprintln};
use nitrolaunch::{
	instance::update::manager::UpdateSettings,
	shared::{minecraft::VersionType, UpdateDepth},
};
use reqwest::Client;

use crate::{
	commands::{call_plugin_subcommand, CmdData},
	output::HYPHEN_POINT,
};

#[derive(Debug, Subcommand)]
pub enum VersionSubcommand {
	#[command(about = "Try a new Minecraft version")]
	List {
		/// Whether to include all versions. Otherwise, only the most recent few will be shown
		#[arg(short, long)]
		all: bool,
		/// Whether to only show release versions
		#[arg(short, long)]
		release: bool,
		/// Whether to only show snapshot versions
		#[arg(short, long)]
		snapshot: bool,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(command: VersionSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match command {
		VersionSubcommand::List {
			all,
			release,
			snapshot,
		} => list(data, all, release, snapshot).await,
		VersionSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("version"), data).await
		}
	}
}

async fn list(
	data: &mut CmdData<'_>,
	all: bool,
	release: bool,
	snapshot: bool,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	let client = Client::new();

	let core = config
		.get_core(
			None,
			&UpdateSettings {
				depth: UpdateDepth::Shallow,
				offline_auth: false,
			},
			&client,
			&config.plugins,
			&data.paths,
			data.output,
		)
		.await?;

	let versions = core
		.get_version_manifest(None, UpdateDepth::Full, data.output)
		.await?;

	// Flags
	let all_false = !release && !snapshot;
	let release = release || all_false;
	let snapshot = snapshot || all_false;

	let limit = if all { None } else { Some(30) };

	for version in versions
		.manifest
		.versions
		.iter()
		.take(limit.unwrap_or(versions.manifest.versions.len()))
		.rev()
	{
		match &version.ty {
			VersionType::Release if !release => continue,
			VersionType::Snapshot if !snapshot => continue,
			_ => {}
		}

		cprint!("{HYPHEN_POINT}");
		match &version.ty {
			VersionType::Release => cprint!("[<s,g>Release] "),
			VersionType::Snapshot => cprint!("[<s,y>Snapshot] "),
			VersionType::OldAlpha => cprint!("[<s,b>Old Alpha] "),
			VersionType::OldBeta => cprint!("[<s,b>Old Beta] "),
			VersionType::Other(ty) => cprint!("[<s>{ty}] "),
		}

		cprintln!("{}", version.id);
	}

	Ok(())
}
