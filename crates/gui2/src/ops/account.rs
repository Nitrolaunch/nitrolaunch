use anyhow::Context;
use freya::query::QueriesStorage;
use itertools::Itertools;
use nitrolaunch::{
	config::modifications::{ConfigModification, apply_modifications_and_write},
	config_crate::account::{AccountConfig, AccountVariant},
	core::account::{Account, AccountKind},
};

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

			Ok(config.accounts.iter_accounts().sorted_by_cached_key(|x| x.0.clone()).map(|x| x.1.clone()).collect())
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

#[rustfmt::skip]
simple_mutation!(
	name = LogoutAccount,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move {
			let mut config = back_state.config().await?;

			let account = config
				.accounts
				.get_account_mut(&id)
				.context("Account does not exist")?;
			account.logout(&back_state.paths.core)
		})
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()>
	{
		QueriesStorage::<FetchAccounts>::invalidate_all()
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = CreateAccount,
	ok = (),
	err = anyhow::Error,
	keys = NotEq<Account>,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let account = keys.clone();

		query_spawn(async move {
			let mut raw_config = back_state.raw_config().await?;
			let mut o = back_state.output();

			let variant = match account.0.get_kind() {
				AccountKind::Microsoft { .. } => AccountVariant::Microsoft,
				AccountKind::Demo => AccountVariant::Demo,
				AccountKind::Unknown(ty) => AccountVariant::Unknown(ty.clone()),
			};
			let new_account = AccountConfig::Simple(variant);

			apply_modifications_and_write(
				&mut raw_config,
				vec![ConfigModification::AddAccount(account.0.get_id().to_string(), new_account)],
				&back_state.paths,
				&back_state.plugins,
				&mut o
			).await
		})
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()>
	{
		QueriesStorage::<FetchAccounts>::invalidate_all()
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = DeleteAccount,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let account = keys.clone();

		query_spawn(async move {
			let mut raw_config = back_state.raw_config().await?;
			let mut o = back_state.output();

			apply_modifications_and_write(
				&mut raw_config,
				vec![ConfigModification::RemoveAccount(account.clone())],
				&back_state.paths,
				&back_state.plugins,
				&mut o,
			)
			.await?;

			let mut data = back_state.data();
			if data.current_account.as_deref() == Some(&account) {
				data.current_account = None;
				let _ = data.write(&back_state.paths);
			}

			Ok(())
		})
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		QueriesStorage::<FetchAccounts>::invalidate_all()
	}
);
