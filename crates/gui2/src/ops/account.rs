use anyhow::Context;
use nitrolaunch::core::account::Account;

use crate::{
	dependency::BackDependency, ops::task::Task, prelude::*, simple_mutation, simple_query,
};

simple_query!(
	name = FetchAccounts,
	ok = Vec<Account>,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let mut config = back_state.config().await?;
			let mut o = back_state.output();

			// Ensure all the accounts have their authentication data loaded from disk
			config.accounts.set_offline(true);
			for id in config.accounts.iter_accounts().map(|x| x.0.clone()).collect::<Vec<_>>() {
				let _ = config
					.accounts
					.authenticate_account(&id, &back_state.paths.core, &back_state.client, &mut o)
					.await;
			}

			Ok(config.accounts.iter_accounts().map(|x| x.1.clone()).collect())
		})
	}
);

simple_mutation!(
	name = LoginAccount,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		let task = async move {
			let mut config = back_state.config().await?;
			let mut o = back_state.output();
			o.set_task(Task::LoginAccount);

			let account = config
				.accounts
				.get_account_mut(&id)
				.context("Account does not exist")?;
			let _ = account.logout(&back_state.paths.core);
			config
				.accounts
				.authenticate_account(&id, &back_state.paths.core, &back_state.client, &mut o)
				.await?;

			back_state.invalidate(BackDependency::Accounts);

			Ok(())
		};

		self.back_state
			.register_task(Task::LoginAccount, tokio::spawn(task));

		async { Ok(()) }
	}
);
