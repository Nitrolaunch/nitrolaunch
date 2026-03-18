use std::path::Path;

use super::CmdData;
use crate::commands::call_plugin_subcommand;
use crate::output::{icons_enabled, CHECK, HYPHEN_POINT, STAR};
use anyhow::{bail, Context};
use inquire::Select;
use itertools::Itertools;
use nitrolaunch::config::modifications::{apply_modifications_and_write, ConfigModification};
use nitrolaunch::config::Config;
use nitrolaunch::config_crate::account::{AccountConfig, AccountVariant};
use nitrolaunch::core::account::{AccountID, AccountKind};

use clap::Subcommand;
use color_print::{cformat, cprint, cprintln};
use nitrolaunch::shared::minecraft::{CosmeticState, SkinVariant};
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
	#[command(about = "Switch to another default account")]
	Switch { account: Option<String> },
	#[command(about = "Get current authentication status")]
	Status,
	#[command(about = "Update the passkey for an account")]
	Passkey {
		/// The account to update the passkey for. If not specified, uses the default account
		account: Option<String>,
	},
	#[command(about = "Log in an account")]
	Login {
		/// The account to authenticate. If not specified, uses the default account
		account: Option<String>,
	},
	#[command(about = "Log out an account")]
	Logout {
		/// The account to log out. If not specified, uses the default account
		account: Option<String>,
	},
	#[command(about = "Add new accounts to your config")]
	Add {},
	#[command(about = "Get or set skins and capes")]
	Cosmetic {
		#[command(subcommand)]
		subcommand: CosmeticSubcommand,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

#[derive(Debug, Subcommand)]
pub enum CosmeticSubcommand {
	#[command(about = "List cosmetics")]
	List {
		/// The account to use. If not specified, uses the default account
		account: Option<String>,
	},
	#[command(about = "Upload a new skin to an account")]
	Upload {
		/// The account to use. If not specified, uses the default account
		account: String,
		/// The path to the skin file
		path: String,
		/// Whether this is a slim (Alex-like) skin
		#[arg(long)]
		slim: bool,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: AccountSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		AccountSubcommand::List { raw } => list(data, raw).await,
		AccountSubcommand::Switch { account } => switch(data, account).await,
		AccountSubcommand::Status => status(data).await,
		AccountSubcommand::Passkey { account } => passkey(data, account).await,
		AccountSubcommand::Login { account } => login(data, account).await,
		AccountSubcommand::Logout { account } => logout(data, account).await,
		AccountSubcommand::Add {} => add(data).await,
		AccountSubcommand::Cosmetic { subcommand } => match subcommand {
			CosmeticSubcommand::List { account } => cosmetic_list(data, account).await,
			CosmeticSubcommand::Upload {
				account,
				path,
				slim,
			} => cosmetic_upload(data, account, path, slim).await,
			CosmeticSubcommand::External(args) => {
				call_plugin_subcommand(args, Some("account.cosmetic"), data).await
			}
		},
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

async fn switch(data: &mut CmdData<'_>, account: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let mut raw_config = data.get_raw_config()?;

	let account = pick_account(account, data.config.get())?;
	raw_config.default_account = Some(account.to_string());

	apply_modifications_and_write(
		&mut raw_config,
		Vec::new(),
		&data.paths,
		&data.config.get().plugins,
		data.output,
	)
	.await?;

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

async fn login(data: &mut CmdData<'_>, account: Option<String>) -> anyhow::Result<()> {
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

async fn cosmetic_list(data: &mut CmdData<'_>, account: Option<String>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();
	if let Some(account) = account {
		config.accounts.choose_account(&account)?;
	}

	let client = Client::new();
	let (skins, capes) = config
		.accounts
		.get_cosmetics(&data.paths.core, &client, data.output)
		.await
		.context("Failed to get cosmetics")?;

	if !skins.is_empty() {
		cprintln!("<s,m>Skins:");
		for skin in skins {
			let line = cformat!("<m>{}", skin.cosmetic.id);
			let line = match skin.cosmetic.state {
				CosmeticState::Active => cformat!("{line} <s,g>{CHECK} Selected"),
				CosmeticState::Inactive => line,
			};
			println!("{HYPHEN_POINT}{line}");
		}
	}
	if !capes.is_empty() {
		cprintln!("<s,y>Capes:");
		for cape in capes {
			let line = cformat!("<y>{} - {}", cape.alias, cape.cosmetic.id);
			let line = match cape.cosmetic.state {
				CosmeticState::Active => cformat!("{line} <s,g>{CHECK} Selected"),
				CosmeticState::Inactive => line,
			};
			println!("{HYPHEN_POINT}{line}");
		}
	}

	Ok(())
}

async fn cosmetic_upload(
	data: &mut CmdData<'_>,
	account: String,
	path: String,
	slim: bool,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get_mut();

	let client = Client::new();

	let path = Path::new(&path);
	if !path.exists() {
		bail!("Skin file does not exist");
	}

	let variant = if slim {
		SkinVariant::Slim
	} else {
		SkinVariant::Classic
	};

	config
		.accounts
		.upload_skin(
			&account,
			variant,
			path,
			&data.paths.core,
			&client,
			data.output,
		)
		.await
		.context("Failed to upload skin")?;

	data.output
		.display(MessageContents::Success("Skin uploaded".into()));

	Ok(())
}

/// Pick which account to use
pub fn pick_account(account: Option<String>, config: &Config) -> anyhow::Result<AccountID> {
	if let Some(account) = account {
		Ok(account.into())
	} else {
		let options = config
			.accounts
			.iter_accounts()
			.map(|x| x.0)
			.sorted()
			.collect();
		let selection = Select::new("Choose an account", options)
			.prompt()
			.context("Prompt failed")?;

		Ok(selection.clone())
	}
}
