use crate::commands::call_plugin_subcommand;

use super::CmdData;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: TemplateSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		TemplateSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("template"), data).await
		}
	}
}
