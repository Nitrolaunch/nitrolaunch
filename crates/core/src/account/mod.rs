/// Authentication for different types of account accounts
pub mod auth;
/// Account cosmetics
pub mod cosmetics;
/// Tools for working with UUIDs
pub mod uuid;

use std::{collections::HashMap, ops::Deref, sync::Arc};

use anyhow::{bail, Context};
use nitro_auth::mc::{AccessToken, ClientId, Keypair};
use nitro_shared::{
	minecraft::{Cape, MinecraftUserProfile, Skin, SkinVariant},
	output::NitroOutput,
};
use reqwest::Client;

use crate::Paths;

use self::auth::AuthParameters;

/// ID for an account
pub type AccountID = Arc<str>;

/// A user account that can play the game
#[derive(Debug, Clone)]
pub struct Account {
	/// Type of this account
	pub(crate) kind: AccountKind,
	/// This account's ID
	id: AccountID,
	/// The account's username
	name: Option<String>,
	/// The account's UUID
	uuid: Option<String>,
	/// The account's access token
	access_token: Option<AccessToken>,
	/// The account's public / private key pair
	keypair: Option<Keypair>,
}

/// Type of an account
#[derive(Debug, Clone)]
pub enum AccountKind {
	/// A new Microsoft account, the standard account
	Microsoft {
		/// The Xbox UID of the account
		xbox_uid: Option<String>,
	},
	/// A demo account
	Demo,
	/// An unknown account kind
	Unknown(String),
}

impl Account {
	/// Create a new account
	pub fn new(kind: AccountKind, id: AccountID) -> Self {
		Self {
			kind,
			id,
			name: None,
			uuid: None,
			access_token: None,
			keypair: None,
		}
	}

	/// Get the ID of this account
	pub fn get_id(&self) -> &AccountID {
		&self.id
	}

	/// Get the name of this account
	pub fn get_name(&self) -> Option<&String> {
		self.name.as_ref()
	}

	/// Checks if this account is a Microsoft account
	pub fn is_microsoft(&self) -> bool {
		matches!(self.kind, AccountKind::Microsoft { .. })
	}

	/// Checks if this account is a demo account
	pub fn is_demo(&self) -> bool {
		matches!(self.kind, AccountKind::Demo)
	}

	/// Gets the kind of this account
	pub fn get_kind(&self) -> &AccountKind {
		&self.kind
	}

	/// Set this account's UUID
	pub fn set_uuid(&mut self, uuid: &str) {
		self.uuid = Some(uuid.to_string());
	}

	/// Get the UUID of this account, if it exists
	pub fn get_uuid(&self) -> Option<&String> {
		self.uuid.as_ref()
	}

	/// Get the access token of this account, if it exists
	pub fn get_access_token(&self) -> Option<&AccessToken> {
		self.access_token.as_ref()
	}

	/// Get the Xbox UID of this account, if it exists
	pub fn get_xbox_uid(&self) -> Option<&String> {
		if let AccountKind::Microsoft { xbox_uid } = &self.kind {
			xbox_uid.as_ref()
		} else {
			None
		}
	}

	/// Get the keypair of this account, if it exists
	pub fn get_keypair(&self) -> Option<&Keypair> {
		self.keypair.as_ref()
	}

	/// Validate the account's username. Returns true if the username is valid,
	/// and false if it isn't
	pub fn validate_username(&self) -> bool {
		if let Some(name) = &self.name {
			if name.is_empty() || name.len() > 16 {
				return false;
			}

			for c in name.chars() {
				if !c.is_ascii_alphanumeric() && c != '_' {
					return false;
				}
			}
		}

		true
	}
}

/// List of accounts and AuthState
#[derive(Clone)]
pub struct AccountManager {
	/// The current state of authentication
	state: AuthState,
	/// All configured / available accounts
	accounts: HashMap<AccountID, Account>,
	/// The MS client ID
	ms_client_id: ClientId,
	/// Whether the manager has been set as offline for authentication
	offline: bool,
	/// Custom hooks for plugin injection
	custom_hooks: Option<Arc<dyn AccountManagerHooks>>,
}

/// State of authentication
#[derive(Debug, Clone)]
enum AuthState {
	/// No account is picked / Nitrolaunch is offline
	Offline,
	/// A default account has been selected
	AccountChosen(AccountID),
}

impl AccountManager {
	/// Create a new AccountManager
	pub fn new(ms_client_id: ClientId) -> Self {
		Self {
			state: AuthState::Offline,
			accounts: HashMap::new(),
			ms_client_id,
			offline: false,
			custom_hooks: None,
		}
	}

	/// Add a new account to the manager
	pub fn add_account(&mut self, account: Account) {
		self.add_account_with_id(account.id.clone(), account);
	}

	/// Add a new account to the manager with a different
	/// ID than the account struct has. I don't know why you would need to do this,
	/// but it's an option anyways
	pub fn add_account_with_id(&mut self, account_id: AccountID, account: Account) {
		self.accounts.insert(account_id, account);
	}

	/// Get an account from the manager
	pub fn get_account(&self, account_id: &str) -> Option<&Account> {
		self.accounts.get(account_id)
	}

	/// Get an account from the manager mutably
	pub fn get_account_mut(&mut self, account_id: &str) -> Option<&mut Account> {
		self.accounts.get_mut(account_id)
	}

	/// Checks if an account with an ID exists
	pub fn account_exists(&self, account_id: &str) -> bool {
		self.accounts.contains_key(account_id)
	}

	/// Iterate over accounts and their IDs
	pub fn iter_accounts(&self) -> impl Iterator<Item = (&AccountID, &Account)> {
		self.accounts.iter()
	}

	/// Remove an account with an ID. Will unchoose the account if it is chosen.
	pub fn remove_account(&mut self, account_id: &str) {
		let is_chosen = if let Some(chosen) = self.get_chosen_account() {
			chosen.get_id().deref() == account_id
		} else {
			false
		};
		if is_chosen {
			self.unchoose_account();
		}
		self.accounts.remove(account_id);
	}

	/// Set the chosen account. Fails if the account does not exist.
	/// If the specified account is already chosen and authenticated, then
	/// no change will be made.
	pub fn choose_account(&mut self, account_id: &str) -> anyhow::Result<()> {
		if !self.account_exists(account_id) {
			bail!("Chosen account does not exist");
		}
		self.state = AuthState::AccountChosen(account_id.into());
		Ok(())
	}

	/// Get the currently chosen account ID, if there is one
	pub fn get_chosen_account_id(&self) -> Option<&str> {
		match &self.state {
			AuthState::Offline => None,
			AuthState::AccountChosen(account_id) => Some(account_id),
		}
	}

	/// Get the currently chosen account, if there is one
	pub fn get_chosen_account(&self) -> Option<&Account> {
		match &self.state {
			AuthState::Offline => None,
			AuthState::AccountChosen(account_id) => self.accounts.get(account_id),
		}
	}

	/// Get the currently chosen mutably, if there is one
	pub fn get_chosen_account_mut(&mut self) -> Option<&mut Account> {
		match &self.state {
			AuthState::Offline => None,
			AuthState::AccountChosen(account_id) => self.accounts.get_mut(account_id),
		}
	}

	/// Checks if an account is chosen
	pub fn is_account_chosen(&self) -> bool {
		matches!(self.state, AuthState::AccountChosen(..))
	}

	/// Checks if an account is chosen and it is authenticated
	pub fn is_authenticated(&self) -> bool {
		let Some(account) = self.get_chosen_account() else {
			return false;
		};
		account.is_authenticated()
	}

	/// Ensures that the currently chosen account is authenticated
	pub async fn authenticate(
		&mut self,
		offline: bool,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		if let AuthState::AccountChosen(account_id) = &mut self.state {
			let account = self
				.accounts
				.get_mut(account_id)
				.expect("Account in AuthState does not exist");

			if !account.is_authenticated() || !account.is_auth_valid(paths) {
				let params = AuthParameters {
					req_client: client,
					paths,
					force: false,
					offline,
					client_id: self.ms_client_id.clone(),
					custom_hooks: self.custom_hooks.clone(),
				};
				account.authenticate(params, o).await?;
			}
		}

		Ok(())
	}

	/// Ensures that a specific account is authenticated
	pub async fn authenticate_account(
		&mut self,
		account: &str,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let account = self
			.accounts
			.get_mut(account)
			.context("Account does not exist")?;

		if !account.is_authenticated() || !account.is_auth_valid(paths) {
			let params = AuthParameters {
				req_client: client,
				paths,
				force: false,
				offline: self.offline,
				client_id: self.ms_client_id.clone(),
				custom_hooks: self.custom_hooks.clone(),
			};
			account.authenticate(params, o).await?;
		}

		Ok(())
	}

	/// Gets cosmetics from the currently chosen account. Returns an error if no account is chosen.
	pub async fn get_cosmetics(
		&mut self,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<(Vec<Skin>, Vec<Cape>)> {
		if let AuthState::AccountChosen(account_id) = &mut self.state {
			let account = self
				.accounts
				.get_mut(account_id)
				.expect("Account in AuthState does not exist");

			let params = AuthParameters {
				req_client: client,
				paths,
				force: false,
				offline: self.offline,
				client_id: self.ms_client_id.clone(),
				custom_hooks: self.custom_hooks.clone(),
			};
			account.get_cosmetics(params, o).await
		} else {
			bail!("No account chosen")
		}
	}

	/// Gets cosmetics from a specific account
	pub async fn get_account_cosmetics(
		&mut self,
		account: &str,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<(Vec<Skin>, Vec<Cape>)> {
		let account = self
			.accounts
			.get_mut(account)
			.context("Account does not exist")?;

		let params = AuthParameters {
			req_client: client,
			paths,
			force: false,
			offline: self.offline,
			client_id: self.ms_client_id.clone(),
			custom_hooks: self.custom_hooks.clone(),
		};
		account.get_cosmetics(params, o).await
	}

	/// Uploads a skin for an account
	pub async fn upload_skin(
		&mut self,
		account: &str,
		variant: SkinVariant,
		skin: &[u8],
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let account = self
			.accounts
			.get_mut(account)
			.context("Account does not exist")?;

		let params = AuthParameters {
			req_client: client,
			paths,
			force: false,
			offline: self.offline,
			client_id: self.ms_client_id.clone(),
			custom_hooks: self.custom_hooks.clone(),
		};
		account.upload_skin(variant, skin, params, o).await
	}

	/// Unchooses the current account, if one is chosen
	pub fn unchoose_account(&mut self) {
		self.state = AuthState::Offline;
	}

	/// Adds accounts from another AccountManager, and copies it's authentication state
	pub fn steal_accounts(&mut self, other: &Self) {
		self.accounts.extend(other.accounts.clone());
		self.state = other.state.clone();
	}

	/// Set whether the AccountManager is offline. When offline, authentication won't use remote servers
	/// if possible, and error if it doesn't have enough local information to authenticate
	pub fn set_offline(&mut self, offline: bool) {
		self.offline = offline;
	}

	/// Set the manager's custom hooks
	pub fn set_custom_hooks(&mut self, hooks: Arc<dyn AccountManagerHooks>) {
		self.custom_hooks = Some(hooks);
	}
}

/// Functions for custom handling for unknown account types
#[async_trait::async_trait]
pub trait AccountManagerHooks: Send + Sync {
	/// Authenticate a custom account
	async fn auth(
		&self,
		id: &str,
		account_type: &str,
	) -> anyhow::Result<Option<MinecraftUserProfile>>;

	/// Get cosmetics for a custom account
	async fn get_cosmetics(
		&self,
		id: &str,
		account_type: &str,
	) -> anyhow::Result<(Vec<Skin>, Vec<Cape>)>;
}

/// Validate a Minecraft username
pub fn validate_username(_kind: &AccountKind, name: &str) -> bool {
	if name.is_empty() || name.len() > 16 {
		return false;
	}

	for c in name.chars() {
		if !c.is_ascii_alphanumeric() && c != '_' {
			return false;
		}
	}

	true
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_username_validation() {
		assert!(validate_username(
			&AccountKind::Microsoft { xbox_uid: None },
			"CarbonSmasher"
		));
		assert!(validate_username(&AccountKind::Demo, "12345"));
		assert!(validate_username(
			&AccountKind::Microsoft { xbox_uid: None },
			"Foo_Bar888"
		));
		assert!(!validate_username(
			&AccountKind::Microsoft { xbox_uid: None },
			""
		));
		assert!(!validate_username(
			&AccountKind::Microsoft { xbox_uid: None },
			"ABCDEFGHIJKLMNOPQRS"
		));
		assert!(!validate_username(
			&AccountKind::Microsoft { xbox_uid: None },
			"+++"
		));
	}

	#[test]
	fn test_account_manager() {
		let mut accounts = AccountManager::new(ClientId::new(String::new()));
		let account = Account::new(AccountKind::Demo, "foo".into());
		accounts.add_account(account);
		accounts
			.choose_account("foo")
			.expect("Failed to choose account");
		let account = Account::new(AccountKind::Demo, "bar".into());
		accounts.add_account(account);
		accounts.remove_account("foo");
		assert!(!accounts.is_account_chosen());
		assert!(!accounts.account_exists("foo"));
	}
}
