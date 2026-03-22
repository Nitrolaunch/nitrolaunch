use anyhow::Context;
use nitro_shared::{
	minecraft::{Cape, Skin, SkinVariant},
	output::NitroOutput,
};

use crate::account::{
	auth::{update_microsoft_account_auth, AuthParameters},
	Account, AccountKind,
};

impl Account {
	/// Get cosmetics for this account
	pub(crate) async fn get_cosmetics(
		&self,
		params: AuthParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<(Vec<Skin>, Vec<Cape>)> {
		match &self.kind {
			AccountKind::Demo => Ok((Vec::new(), Vec::new())),
			AccountKind::Microsoft { .. } => {
				let client = params.req_client.clone();
				let account_data = update_microsoft_account_auth(&self.id, params, o)
					.await
					.context("Failed to update account authentication")?;
				// If we just authenticated, use the profile we already downloaded. If not, get it.
				let profile = if account_data.profile.skins.is_empty() {
					crate::net::minecraft::get_user_profile(&account_data.access_token.0, &client)
						.await?
				} else {
					account_data.profile
				};

				Ok((profile.skins, profile.capes))
			}
			AccountKind::Unknown(ty) => {
				if let Some(hooks) = params.custom_hooks {
					hooks.get_cosmetics(&self.id, ty).await
				} else {
					Ok((Vec::new(), Vec::new()))
				}
			}
		}
	}

	/// Uploads a skin to this account
	pub(crate) async fn upload_skin(
		&mut self,
		variant: SkinVariant,
		skin: &[u8],
		params: AuthParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		match &self.kind {
			AccountKind::Demo => Ok(()),
			AccountKind::Microsoft { .. } => {
				let client = params.req_client.clone();
				self.authenticate(params, o).await?;

				crate::net::minecraft::upload_skin(
					variant,
					skin,
					self.access_token.as_ref().map(|x| x.0.as_str()).unwrap(),
					&client,
				)
				.await?;

				Ok(())
			}
			AccountKind::Unknown(..) => Ok(()),
		}
	}
}
