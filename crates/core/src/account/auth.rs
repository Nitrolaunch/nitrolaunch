use std::sync::Arc;

use anyhow::{bail, Context};
use nitro_auth::RsaPrivateKey;
use nitro_shared::minecraft::MinecraftUserProfile;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::translate;
use nitro_shared::util::utc_timestamp;

use crate::Paths;
use nitro_auth::db::{AuthDatabase, DatabaseAccount, SensitiveAccountInfo};
use nitro_auth::mc::Keypair;
use nitro_auth::mc::{
	self as auth, authenticate_microsoft_account, authenticate_microsoft_account_from_token,
	AccessToken, ClientId, RefreshToken,
};

use super::{Account, AccountKind, CustomAuthFunction};

impl Account {
	/// Authenticate the account
	pub(crate) async fn authenticate(
		&mut self,
		params: AuthParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		match &mut self.kind {
			AccountKind::Microsoft { xbox_uid } => {
				if params.offline {
					let db = AuthDatabase::open(&params.paths.auth)
						.context("Failed to open authentication database")?;
					let Some((account, sensitive)) = get_full_account(&db, &self.id, o)
						.await
						.context("Failed to get account from database")?
					else {
						bail!("Account not present in database. Make sure to authenticate at least once before logging in in offline mode");
					};

					self.name = Some(account.username.clone());
					self.uuid = Some(account.uuid.clone());
					self.keypair = sensitive.keypair.clone();
					*xbox_uid = sensitive.xbox_uid.clone();
				} else {
					let account_data = update_microsoft_account_auth(&self.id, params, o)
						.await
						.context("Failed to update account authentication")?;

					self.access_token = Some(account_data.access_token);
					self.name = Some(account_data.profile.name);
					self.uuid = Some(account_data.profile.uuid);
					self.keypair = account_data.keypair;
					*xbox_uid = account_data.xbox_uid;
				}
			}
			AccountKind::Demo => {}
			AccountKind::Unknown(other) => {
				if let Some(func) = params.custom_auth_fn {
					o.display(
						MessageContents::Simple(
							"Handling custom account type with authentication function".into(),
						),
						MessageLevel::Debug,
					);
					let profile = func
						.auth(&self.id, other)
						.await
						.context("Custom auth function failed")?;
					if let Some(profile) = profile {
						self.name = Some(profile.name);
						self.uuid = Some(profile.uuid);
					}
				} else {
					o.display(
						MessageContents::Simple(
							"Authentication for custom account type not handled".into(),
						),
						MessageLevel::Debug,
					);
				}
			}
		}

		Ok(())
	}

	/// Checks if the account still has valid authentication. This does not mean that they are
	/// authenticated yet. To check if the account is authenticated and ready to be used, use the is_authenticated
	/// function instead.
	pub fn is_auth_valid(&self, paths: &Paths) -> bool {
		match &self.kind {
			AccountKind::Microsoft { .. } => {
				let Ok(db) = AuthDatabase::open(&paths.auth) else {
					return false;
				};

				db.get_valid_account(&self.id).is_some()
			}
			AccountKind::Demo => true,
			AccountKind::Unknown(..) => true,
		}
	}

	/// Checks if this account is currently authenticated and ready to be used
	pub fn is_authenticated(&self) -> bool {
		match &self.kind {
			AccountKind::Microsoft { .. } => self.access_token.is_some() && self.uuid.is_some(),
			AccountKind::Demo => true,
			AccountKind::Unknown(..) => true,
		}
	}

	/// Updates this account's passkey using prompts
	pub async fn update_passkey(
		&self,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let mut db =
			AuthDatabase::open(&paths.auth).context("Failed to open authentication database")?;
		let account = db.get_account_mut(&self.id).context(
			"Account does not exist in database. Try authenticating first before setting a passkey",
		)?;
		let old_passkey = if account.has_passkey() {
			Some(
				o.prompt_password(MessageContents::Simple(format!(
					"Enter the old passkey for account '{}'",
					self.id
				)))
				.await
				.context("Failed to get old passkey")?,
			)
		} else {
			None
		};
		let new_passkey = o
			.prompt_new_password(MessageContents::Simple(format!(
				"Enter the new passkey for account '{}'",
				self.id
			)))
			.await
			.context("Failed to get new passkey")?;
		account
			.update_passkey(old_passkey.as_deref(), &new_passkey)
			.context("Failed to update passkey for account")?;

		db.write()
			.context("Failed to write to authentication database")?;

		Ok(())
	}

	/// Logs out this account and removes their data from the auth database (not including passkey)
	pub fn logout(&mut self, paths: &Paths) -> anyhow::Result<()> {
		let mut db =
			AuthDatabase::open(&paths.auth).context("Failed to open authentication database")?;
		db.logout_account(&self.id)
			.context("Failed to logout account in database")?;

		db.write()
			.context("Failed to write authentication database")?;

		Ok(())
	}
}

/// Data for a Microsoft account
pub struct MicrosoftAccountData {
	access_token: AccessToken,
	profile: MinecraftUserProfile,
	xbox_uid: Option<String>,
	keypair: Option<Keypair>,
}

/// Updates authentication for a Microsoft account using either the database or updating from the API
async fn update_microsoft_account_auth(
	account_id: &str,
	params: AuthParameters<'_>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<MicrosoftAccountData> {
	let mut db =
		AuthDatabase::open(&params.paths.auth).context("Failed to open authentication database")?;

	// Force reauth if specified
	if params.force {
		return reauth_microsoft_account(
			account_id,
			&mut db,
			params.client_id,
			params.req_client,
			o,
		)
		.await;
	}

	// Check the authentication DB
	let account_data = if let Some((db_account, sensitive)) = get_full_account(&db, account_id, o)
		.await
		.context("Failed to get full account from database")?
	{
		let db_account = db_account.clone();

		// See if we have a non-expired access token already stored
		let access_token = if let (Some(access_token), Some(expiration)) =
			(&sensitive.access_token, &sensitive.access_token_expires)
		{
			if utc_timestamp().unwrap_or(u64::MAX) < *expiration {
				AccessToken(access_token.clone())
			} else {
				update_using_refresh_token(account_id, &sensitive, &params, &mut db, o)
					.await
					.context("Failed to refresh authentication")?
			}
		} else {
			update_using_refresh_token(account_id, &sensitive, &params, &mut db, o)
				.await
				.context("Failed to refresh authentication")?
		};

		MicrosoftAccountData {
			access_token,
			profile: MinecraftUserProfile {
				name: db_account.username.clone(),
				uuid: db_account.uuid.clone(),
				skins: Vec::new(),
				capes: Vec::new(),
			},
			xbox_uid: sensitive.xbox_uid.clone(),
			keypair: sensitive.keypair.clone(),
		}
	} else {
		// Authenticate with the server again
		reauth_microsoft_account(account_id, &mut db, params.client_id, params.req_client, o)
			.await?
	};

	Ok(account_data)
}

/// Gets the access token using the refresh token
async fn update_using_refresh_token(
	account_id: &str,
	sensitive: &SensitiveAccountInfo,
	params: &AuthParameters<'_>,
	db: &mut AuthDatabase,
	o: &mut impl NitroOutput,
) -> anyhow::Result<AccessToken> {
	let refresh_token = RefreshToken::new(
		sensitive
			.refresh_token
			.clone()
			.expect("Refresh token should be present in a full valid account"),
	);
	// Get the access token using the refresh token
	let oauth_client =
		auth::create_client(params.client_id.clone()).context("Failed to create OAuth client")?;
	let token = auth::refresh_microsoft_token(&oauth_client, &refresh_token)
		.await
		.context("Failed to get refreshed token")?;

	let auth_result = authenticate_microsoft_account_from_token(token, params.req_client, o)
		.await
		.context("Failed to authenticate with refreshed token")?;

	let mut db_account = db
		.get_account(account_id)
		.context("Failed to get account from database")?
		.clone();

	let mut sensitive = get_sensitive_info(&db_account, o)
		.await
		.context("Failed to get sensitive info")?;
	sensitive.access_token = Some(auth_result.access_token.0.clone());
	sensitive.access_token_expires = utc_timestamp().map(|x| x + 24 * 3600).ok();
	db_account
		.set_sensitive_info(sensitive)
		.context("Failed to set sensitive info for account")?;

	db.update_account(db_account, account_id)
		.context("Failed to update account in database")?;

	Ok(AccessToken(auth_result.access_token.0.clone()))
}

/// Fully reauthenticates, getting a new refresh token using a login
async fn reauth_microsoft_account(
	account_id: &str,
	db: &mut AuthDatabase,
	client_id: ClientId,
	client: &reqwest::Client,
	o: &mut impl NitroOutput,
) -> anyhow::Result<MicrosoftAccountData> {
	let auth_result = authenticate_microsoft_account(client_id, client, o)
		.await
		.context("Failed to authenticate account")?;

	let ownership_task = {
		let client = client.clone();
		let token = auth_result.access_token.0.clone();
		async move {
			let owns_game = auth::account_owns_game(&token, &client)
				.await
				.context("Failed to check for game ownership")?;

			Ok::<bool, anyhow::Error>(owns_game)
		}
	};

	let profile_task = {
		let client = client.clone();
		let token = auth_result.access_token.0.clone();
		async move {
			let profile = crate::net::minecraft::get_user_profile(&token, &client)
				.await
				.context("Failed to get Microsoft account profile")?;

			Ok::<MinecraftUserProfile, anyhow::Error>(profile)
		}
	};

	let certificate_task = {
		let client = client.clone();
		let token = auth_result.access_token.0.clone();
		async move {
			let certificate = crate::net::minecraft::get_user_certificate(&token, &client)
				.await
				.context("Failed to get user certificate")?;

			Ok(certificate)
		}
	};

	let (owns_game, profile, certificate) =
		tokio::try_join!(ownership_task, profile_task, certificate_task)?;

	if !owns_game {
		bail!("Specified account does not own Minecraft");
	}

	// Calculate expiration time
	let expiration_time = nitro_auth::db::calculate_expiration_date();

	// Write the new account to the database

	let sensitive = SensitiveAccountInfo {
		refresh_token: auth_result.refresh_token.map(|x| x.secret().clone()),
		xbox_uid: Some(auth_result.xbox_uid.clone()),
		keypair: Some(certificate.key_pair.clone()),
		access_token: Some(auth_result.access_token.0.clone()),
		// Expires in 24 hours
		access_token_expires: utc_timestamp().map(|x| x + 24 * 3600).ok(),
	};
	let db_account = DatabaseAccount::new(
		account_id.to_string(),
		profile.name.clone(),
		profile.uuid.clone(),
		expiration_time,
		sensitive,
	)
	.context("Failed to create new account in database")?;

	db.update_account(db_account, account_id)
		.context("Failed to update account in database")?;

	Ok(MicrosoftAccountData {
		access_token: auth_result.access_token,
		xbox_uid: Some(auth_result.xbox_uid),
		profile,
		keypair: Some(certificate.key_pair),
	})
}

/// Tries to get a full valid account from the database along with a passkey prompt if applicable
async fn get_full_account<'db>(
	db: &'db AuthDatabase,
	account_id: &str,
	o: &mut impl NitroOutput,
) -> anyhow::Result<Option<(&'db DatabaseAccount, SensitiveAccountInfo)>> {
	let Some(account) = db.get_valid_account(account_id) else {
		return Ok(None);
	};
	// We have to reauthenticate non-logged-in accounts
	if !account.is_logged_in() {
		return Ok(None);
	}

	// Get their sensitive info
	let sensitive = get_sensitive_info(account, o)
		.await
		.context("Failed to get sensitive information")?;
	if sensitive.refresh_token.is_none() {
		return Ok(None);
	}

	Ok(Some((account, sensitive)))
}

/// Gets sensitive info from an account using their passkey
async fn get_sensitive_info(
	db_account: &DatabaseAccount,
	o: &mut impl NitroOutput,
) -> anyhow::Result<SensitiveAccountInfo> {
	let out = if db_account.has_passkey() {
		let private_key = get_private_key(
			db_account,
			MessageContents::Simple(format!(
				"Please enter the passkey for the account '{}'",
				db_account.id
			)),
			o,
		)
		.await
		.context("Failed to get key")?;

		let out = db_account
			.get_sensitive_info_with_key(&private_key)
			.context("Failed to get sensitive account info using key")?;
		o.display(
			MessageContents::Success(translate!(o, PasskeyAccepted)),
			MessageLevel::Important,
		);

		out
	} else {
		db_account
			.get_sensitive_info_no_passkey()
			.context("Failed to get sensitive account info without key")?
	};

	Ok(out)
}

/// Gets the account's private key with a repeating passkey prompt.
/// The account must have a passkey available.
async fn get_private_key(
	account: &DatabaseAccount,
	message: MessageContents,
	o: &mut impl NitroOutput,
) -> anyhow::Result<RsaPrivateKey> {
	const MAX_ATTEMPTS: u8 = 3;

	for _ in 0..MAX_ATTEMPTS {
		let result = o
			.prompt_special_account_passkey(message.clone(), &account.id)
			.await;
		if let Ok(passkey) = result {
			let result = account.get_private_key(&passkey);
			match result {
				Ok(private_key) => {
					return Ok(private_key.expect("Account should have passkey"));
				}
				Err(e) => {
					o.display(
						MessageContents::Error(format!("{e:?}")),
						MessageLevel::Important,
					);
				}
			}
		}
	}

	bail!("Passkey authentication failed; max attempts exceeded")
}

/// Container struct for parameters for authenticating an account
pub(crate) struct AuthParameters<'a> {
	pub force: bool,
	pub offline: bool,
	pub client_id: ClientId,
	pub paths: &'a Paths,
	pub req_client: &'a reqwest::Client,
	pub custom_auth_fn: Option<Arc<dyn CustomAuthFunction>>,
}

/// Checks whether an account in the database is logged in
pub fn check_game_ownership(paths: &Paths) -> anyhow::Result<bool> {
	let db = AuthDatabase::open(&paths.auth).context("Failed to open auth database")?;

	Ok(db.has_logged_in_account())
}
