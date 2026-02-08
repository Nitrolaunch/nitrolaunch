use super::CmdData;
use crate::commands::call_plugin_subcommand;
use crate::output::{icons_enabled, HYPHEN_POINT, STAR};
use anyhow::{bail, Context};
use itertools::Itertools;
use nitrolaunch::config::modifications::{apply_modifications_and_write, ConfigModification};
use nitrolaunch::config_crate::account::{AccountConfig, AccountVariant};
use nitrolaunch::core::account::AccountKind;

use clap::Subcommand;
use color_print::{cprint, cprintln};
use nitrolaunch::shared::output::{MessageContents, NitroOutput};
use reqwest::Client;

#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
	#[command(about = "List all accounts")]
	#[clap(alias = "ls")]
	List {
		/// Whether to remove formatting and warnings from the output
		#[arg(short, long)]
		raw: bool,
	},
	#[command(about = "Get current authentication status")]
	Status,
	#[command(about = "Update the passkey for an account")]
	Passkey {
		/// The account to update the passkey for. If not specified, uses the default account
		#[arg(short, long)]
		account: Option<String>,
	},
	#[command(about = "Ensure that an account is authenticated")]
	Auth {
		/// The account to authenticate. If not specified, uses the default account
		#[arg(short, long)]
		account: Option<String>,
	},
	#[command(about = "Log out an account")]
	Logout {
		/// The account to log out. If not specified, uses the default account
		#[arg(short, long)]
		account: Option<String>,
	},
	#[command(about = "Add new accounts to your config")]
	Add {},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: AccountSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		AccountSubcommand::List { raw } => list(data, raw).await,
		AccountSubcommand::Status => status(data).await,
		AccountSubcommand::Passkey { account } => passkey(data, account).await,
		AccountSubcommand::Auth { account } => auth(data, account).await,
		AccountSubcommand::Logout { account } => logout(data, account).await,
		AccountSubcommand::Add {} => add(data).await,
		AccountSubcommand::External(args) => {
			call_plugin_subcommand(args, Some("account"), data).await
		}
	}
}

async fn list(data: &mut CmdData<'_>, raw: bool) -> anyhow::Result<()> {
	data.ensure_config(!raw).await?;
	let config = data.config.get();

	if !raw {
		cprintln!("<s>Accounts:");
	}
	for (id, account) in config.accounts.iter_accounts().sorted_by_key(|x| x.0) {
		cprint!("{}", HYPHEN_POINT);
		if raw {
			println!("{id}");
		} else {
			match account.get_kind() {
				AccountKind::Microsoft { .. } => {
					cprint!("<s><g>{}</g>", id)
				}
				AccountKind::Demo => cprint!("<s><c!>{}</c!>", id),
				AccountKind::Unknown(other) => cprint!("<s><k!>({other}) {}</k!>", id),
			}
			if let Some(chosen) = config.accounts.get_chosen_account() {
				if chosen.get_id() == id {
					if icons_enabled() {
						cprint!("<y> {}", STAR);
					} else {
						cprint!("<s> (Default)");
					}
				}
			}
			println!();
		}
	}

	Ok(())
}

async fn status(data: &mut CmdData<'_>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	match config.accounts.get_chosen_account() {
		Some(account) => {
			let account_valid = account.is_auth_valid(&data.paths.core);
			if account_valid {
				cprint!("<g>Logged in as ");
			} else {
				cprint!("<g>Account chosen as ");
			}
			match account.get_kind() {
				AccountKind::Microsoft { .. } => cprint!("<s,g!>{}", account.get_id()),
				AccountKind::Demo => cprint!("<s,c!>{}", account.get_id()),
				AccountKind::Unknown(other) => cprint!("<s,k!>({other}) {}", account.get_id()),
			}

			if !account_valid {
				cprint!(" - <r>Currently logged out");
			}
			cprintln!();
		}
		None => cprintln!("<r>No account chosen"),
	}

	Ok(())
}

async fn passkey(data: &mut CmdData<'_>, account: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();
	let account = if let Some(account) = account {
		config.accounts.get_account(&account)
	} else {
		config.accounts.get_chosen_account()
	};
	let Some(account) = account else {
		bail!("Specified account does not exist");
	};

	account
		.update_passkey(&data.paths.core, data.output)
		.await
		.context("Failed to update passkey")?;

	Ok(())
}

async fn auth(data: &mut CmdData<'_>, account: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();
	if let Some(account) = account {
		config.accounts.choose_account(&account)?;
	}

	let client = Client::new();
	config
		.accounts
		.authenticate(&data.paths.core, &client, data.output)
		.await
		.context("Failed to authenticate")?;

	Ok(())
}

async fn logout(data: &mut CmdData<'_>, account: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();
	let account = if let Some(account) = account {
		config.accounts.get_account_mut(&account)
	} else {
		config.accounts.get_chosen_account_mut()
	};
	let Some(account) = account else {
		bail!("Specified account does not exist");
	};

	account
		.logout(&data.paths.core)
		.context("Failed to logout account")?;

	Ok(())
}

async fn add(data: &mut CmdData<'_>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut config = data.get_raw_config()?;

	// Build the account
	let id = inquire::Text::new("What is the ID for the account?").prompt()?;

	let options = vec![AccountVariant::Microsoft {}, AccountVariant::Demo {}];
	let kind = inquire::Select::new("What kind of account is this?", options).prompt()?;

	let account = AccountConfig::Simple(kind);

	apply_modifications_and_write(
		&mut config,
		vec![ConfigModification::AddAccount(id, account)],
		&data.paths,
		&data.config.get().plugins,
		data.output,
	)
	.await
	.context("Failed to write modified config")?;

	data.output
		.display(MessageContents::Success("Account added".into()));

	Ok(())
}
