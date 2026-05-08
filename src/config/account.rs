use anyhow::Context;
use nitro_config::account::{AccountConfig, AccountVariant};
use nitro_core::account::{Account, AccountKind, AccountManagerHooks};
use nitro_plugin::hook::hooks::{
	ActivateCape, ActivateCapeArg, GetAccountCosmetics, GetAccountCosmeticsArg, HandleAuth,
	HandleAuthArg, UploadSkin, UploadSkinArg,
};
use nitro_shared::{
	minecraft::{Cape, MinecraftUserProfile, Skin, SkinVariant},
	output::NoOp,
};

use crate::{io::paths::Paths, plugin::PluginManager};

/// Creates an account from an account config
pub fn read_account_config(config: &AccountConfig, id: &str) -> Account {
	match config {
		AccountConfig::Simple(variant) | AccountConfig::Advanced { variant } => {
			let kind = match variant {
				AccountVariant::Microsoft => AccountKind::Microsoft { xbox_uid: None },
				AccountVariant::Demo => AccountKind::Demo,
				AccountVariant::Unknown(id) => AccountKind::Unknown(id.clone()),
			};
			Account::new(kind, id.into())
		}
	}
}

/// AccountManagerHooks implementation for account types using plugins
pub struct AuthFunction {
	pub plugins: PluginManager,
	pub paths: Paths,
}

#[async_trait::async_trait]
impl AccountManagerHooks for AuthFunction {
	async fn auth(
		&self,
		id: &str,
		account_type: &str,
	) -> anyhow::Result<Option<MinecraftUserProfile>> {
		let arg = HandleAuthArg {
			account_id: id.to_string(),
			account_type: account_type.to_string(),
		};
		let mut results = self
			.plugins
			.call_hook(HandleAuth, &arg, &self.paths, &mut NoOp)
			.await
			.context("Failed to call handle auth hook")?;

		let mut out = None;
		while let Some(result) = results.next_result(&mut NoOp).await? {
			if result.handled {
				out = result.profile;
			}
		}

		Ok(out)
	}

	async fn get_cosmetics(
		&self,
		id: &str,
		account_type: &str,
	) -> anyhow::Result<(Vec<Skin>, Vec<Cape>)> {
		let arg = GetAccountCosmeticsArg {
			id: id.to_string(),
			kind: account_type.to_string(),
		};
		let mut results = self
			.plugins
			.call_hook(GetAccountCosmetics, &arg, &self.paths, &mut NoOp)
			.await
			.context("Failed to call get cosmetics hook")?;

		let mut skins = Vec::new();
		let mut capes = Vec::new();
		while let Some(result) = results.next_result(&mut NoOp).await? {
			skins.extend(result.skins);
			capes.extend(result.capes);
		}

		Ok((skins, capes))
	}

	async fn upload_skin(
		&self,
		id: &str,
		account_type: &str,
		skin: &[u8],
		variant: SkinVariant,
	) -> anyhow::Result<()> {
		let arg = UploadSkinArg {
			id: id.to_string(),
			kind: account_type.to_string(),
			data: skin.to_vec(),
			variant,
		};

		let results = self
			.plugins
			.call_hook(UploadSkin, &arg, &self.paths, &mut NoOp)
			.await
			.context("Failed to call upload skin hook")?;

		results.all_results(&mut NoOp).await?;

		Ok(())
	}

	async fn activate_cape(
		&self,
		id: &str,
		account_type: &str,
		cape: Option<&str>,
	) -> anyhow::Result<()> {
		let arg = ActivateCapeArg {
			id: id.to_string(),
			kind: account_type.to_string(),
			cape: cape.map(|x| x.to_string()),
		};

		let results = self
			.plugins
			.call_hook(ActivateCape, &arg, &self.paths, &mut NoOp)
			.await
			.context("Failed to call activate cape hook")?;

		results.all_results(&mut NoOp).await?;

		Ok(())
	}
}
