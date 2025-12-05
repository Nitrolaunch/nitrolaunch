use crate::{
	commands::call_plugin_subcommand,
	output::{icons_enabled, HYPHEN_POINT, INSTANCE, LOADER, PACKAGE, VERSION},
};

use super::CmdData;

use anyhow::Context;
use clap::Subcommand;
use color_print::{cprint, cprintln};
use inquire::Select;
use itertools::Itertools;
use nitrolaunch::{
	config::Config,
	config_crate::template::TemplateLoaderConfiguration,
	core::util::versions::MinecraftVersion,
	shared::{id::TemplateID, Side},
};

#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
	#[command(about = "List all instances")]
	#[clap(alias = "ls")]
	List {
		/// Whether to remove formatting and warnings from the output
		#[arg(short, long)]
		raw: bool,
	},
	#[command(about = "Print useful information about an instance")]
	Info { instance: Option<String> },
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: TemplateSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		TemplateSubcommand::List { raw } => list(data, raw).await,
		TemplateSubcommand::Info { instance } => info(data, instance).await,
		TemplateSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("template"), data).await
		}
	}
}

async fn list(data: &mut CmdData<'_>, raw: bool) -> anyhow::Result<()> {
	data.ensure_config(!raw).await?;
	let config = data.config.get_mut();

	for (id, _) in config.templates.iter().sorted_by_key(|x| x.0) {
		if raw {
			println!("{id}");
		} else {
			cprintln!("{}<y!>{}", HYPHEN_POINT, id);
		}
	}

	Ok(())
}

async fn info(data: &mut CmdData<'_>, id: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let id = pick_template(id, config)?;

	fn print_indent() {
		print!("   ");
	}

	let template = config
		.templates
		.get(&id)
		.with_context(|| format!("Unknown template '{id}'"))?;

	if icons_enabled() {
		print!("{} ", INSTANCE);
	}
	cprintln!("<s><g>Template <b>{}", id);
	if let Some(version) = &template.instance.version {
		print_indent();
		if icons_enabled() {
			print!("{} ", VERSION);
		}
		cprintln!(
			"<s>Version:</s> <g>{}",
			MinecraftVersion::from_deser(version)
		);
	}

	if let Some(side) = template.instance.side {
		print_indent();
		cprint!("{}Type: ", HYPHEN_POINT);
		match side {
			Side::Client => cprintln!("<y!>Client"),
			Side::Server => cprintln!("<c!>Server"),
		}
	}

	print_indent();
	if icons_enabled() {
		print!("{} ", LOADER);
	}
	match &template.loader {
		TemplateLoaderConfiguration::Simple(Some(loader)) => {
			cprintln!("<s>Loader:</s> <g>{loader}");
		}
		TemplateLoaderConfiguration::Full { client, server } => {
			if let Some(loader) = client {
				cprintln!("<s>Client Loader:</s> <g>{loader}");
			}
			if let Some(loader) = server {
				cprintln!("<s>Server Loader:</s> <g>{loader}");
			}
		}
		_ => {}
	}

	print_indent();
	if icons_enabled() {
		print!("{} ", PACKAGE);
	}
	cprintln!("<s>Packages:");
	cprintln!("   <s>Global Packages:");
	for pkg in template.packages.iter_global() {
		print_indent();
		cprint!("{}", HYPHEN_POINT);
		cprint!("<b!>{}<g!>", pkg.get_pkg_id());
		cprintln!();
	}
	cprintln!("   <s>Client Packages:");
	for pkg in template.packages.iter_side(Side::Client) {
		print_indent();
		cprint!("{}", HYPHEN_POINT);
		cprint!("<b!>{}<g!>", pkg.get_pkg_id());
		cprintln!();
	}
	cprintln!("   <s>Server Packages:");
	for pkg in template.packages.iter_side(Side::Server) {
		print_indent();
		cprint!("{}", HYPHEN_POINT);
		cprint!("<b!>{}<g!>", pkg.get_pkg_id());
		cprintln!();
	}

	Ok(())
}

/// Pick which template to use
pub fn pick_template(template: Option<String>, config: &Config) -> anyhow::Result<TemplateID> {
	if let Some(template) = template {
		Ok(template.into())
	} else {
		let options = config.templates.keys().sorted().collect();
		let selection = Select::new("Choose a template", options)
			.prompt()
			.context("Prompt failed")?;

		Ok(selection.to_owned())
	}
}
