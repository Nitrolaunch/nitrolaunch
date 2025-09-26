use crate::commands::call_plugin_subcommand;

use super::CmdData;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum ProfileSubcommand {
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: ProfileSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		ProfileSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("profile"), data).await
		}
	}
}
