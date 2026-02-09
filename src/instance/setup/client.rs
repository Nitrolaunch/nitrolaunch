use anyhow::Context;
use nitro_core::account::AccountManager;

use super::super::update::manager::UpdateMethodResult;
use super::{InstKind, Instance};

impl Instance {
	/// Set up data for a client
	pub async fn setup_client(
		&mut self,
		accounts: &AccountManager,
	) -> anyhow::Result<UpdateMethodResult> {
		debug_assert!(matches!(self.kind, InstKind::Client { .. }));

		let out = UpdateMethodResult::new();
		self.ensure_dir()?;

		// Create keypair file
		if accounts.is_authenticated() {
			if let Some(account) = accounts.get_chosen_account() {
				self.create_keypair(account)
					.context("Failed to create account keypair")?;
			}
		}

		Ok(out)
	}
}
