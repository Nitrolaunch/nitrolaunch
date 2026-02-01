use anyhow::Context;
use nitro_core::account::AccountManager;

use crate::io::paths::Paths;

use super::super::update::manager::UpdateMethodResult;
use super::{InstKind, Instance};

impl Instance {
	/// Set up data for a client
	pub async fn setup_client(
		&mut self,
		paths: &Paths,
		accounts: &AccountManager,
	) -> anyhow::Result<UpdateMethodResult> {
		debug_assert!(matches!(self.kind, InstKind::Client { .. }));

		let out = UpdateMethodResult::new();
		self.ensure_dirs(paths)?;

		// Create keypair file
		if accounts.is_authenticated() {
			if let Some(account) = accounts.get_chosen_account() {
				self.create_keypair(account, paths)
					.context("Failed to create account keypair")?;
			}
		}

		Ok(out)
	}
}
