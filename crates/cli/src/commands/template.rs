use crate::{
	commands::{call_plugin_subcommand, config::edit_temp_file},
	output::{icons_enabled, HYPHEN_POINT, INSTANCE, LOADER, PACKAGE, VERSION},
};
use std::ops::DerefMut;

use super::CmdData;

use anyhow::{bail, Context};
use clap::Subcommand;
use color_print::{cprint, cprintln};
use inquire::{Confirm, Select};
use itertools::Itertools;
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::template::{TemplateConfig, TemplateLoaderConfiguration},
	core::util::versions::MinecraftVersion,
	plugin_crate::hook::hooks::DeleteTemplate,
	shared::{
		id::TemplateID,
		output::{MessageContents, MessageLevel, NitroOutput},
		Side,
	},
};

#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
	#[command(about = "List all templates")]
	#[clap(alias = "ls")]
	List {
		/// Whether to remove formatting and warnings from the output
		#[arg(short, long)]
		raw: bool,
	},
	#[command(about = "Print useful information about a template")]
	Info { template: Option<String> },
	#[command(about = "Edit configuration for a template")]
	Edit {
		/// The template to edit
		template: Option<String>,
	},
	#[command(about = "Delete a template forever")]
	Delete {
		/// The template to delete
		template: Option<String>,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: TemplateSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		TemplateSubcommand::List { raw } => list(data, raw).await,
		TemplateSubcommand::Info { template } => info(data, template).await,
		TemplateSubcommand::Delete { template } => delete(data, template).await,
		TemplateSubcommand::Edit { template } => edit(data, template).await,
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
			cprintln!("{}<b>{}", HYPHEN_POINT, id);
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

async fn delete(data: &mut CmdData<'_>, id: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut raw_config = data.get_raw_config()?;
	let config = data.config.get_mut();

	let id = pick_template(id, config)?;

	let template = config
		.templates
		.get(&id)
		.with_context(|| format!("Unknown template '{id}'"))?;

	let prompt = Confirm::new("Are you SURE you want to delete this template? (y/n)");
	if !prompt.prompt()? {
		cprintln!("<r>Cancelled.");
		return Ok(());
	}

	let mut process = data.output.get_process();
	process.display(
		MessageContents::StartProcess("Deleting template".into()),
		MessageLevel::Important,
	);

	if let Some(source_plugin) = &template.instance.source_plugin {
		if !template.instance.is_deletable {
			bail!("Plugin template does not support deletion");
		}

		let result = config
			.plugins
			.call_hook_on_plugin(
				DeleteTemplate,
				source_plugin,
				&id.to_string(),
				&data.paths,
				process.deref_mut(),
			)
			.await?;
		if let Some(result) = result {
			result.result(process.deref_mut()).await?;
		}
	} else {
		let modifications = vec![ConfigModification::RemoveTemplate(id.into())];
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
		MessageContents::Success("Template deleted".into()),
		MessageLevel::Important,
	);

	Ok(())
}

async fn edit(data: &mut CmdData<'_>, id: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut raw_config = data.get_raw_config()?;
	let config = data.config.get_mut();

	let id = pick_template(id, config)?;

	let template = config
		.templates
		.get_mut(&id)
		.with_context(|| format!("Unknown template '{id}'"))?;

	let mut temp_config = template.clone();
	if temp_config.instance.source_plugin.is_some() && !temp_config.instance.is_editable {
		bail!("This plugin template does not support editing");
	}
	temp_config.instance.remove_plugin_only_fields();

	let text = serde_json::to_string_pretty(&temp_config).context("Failed to serialize config")?;
	let edited = edit_temp_file(&text, &format!("Editing template {id}"), &data.paths)?;
	let mut new_config: TemplateConfig = serde_json::from_str(&edited)
		.context("Failed to serialize. Make sure your config is valid JSON")?;
	new_config
		.instance
		.restore_plugin_only_fields(&temp_config.instance);

	let modifications = vec![ConfigModification::AddTemplate(id.into(), new_config)];
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
