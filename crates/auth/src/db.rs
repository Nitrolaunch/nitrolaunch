use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context};
use nitro_shared::util::utc_timestamp;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};

use crate::mc::Keypair;
use crate::passkey::{decrypt_chunks, encrypt_chunks};

/// The amount of time to consider a refresh token valid for. We want it to expire eventually to
/// ensure some amount of security.
// 180 days
const REFRESH_TOKEN_EXPIRATION: u64 = 15552000;

/// A handle to the authentication database where things like credentials are stored
pub struct AuthDatabase {
	/// The directory where the database is stored
	dir: PathBuf,
	/// The contents of the main database file
	contents: DatabaseContents,
}

impl AuthDatabase {
	/// Open the database in the specified directory
	pub fn open(path: &Path) -> anyhow::Result<Self> {
		std::fs::create_dir_all(path).context("Failed to ensure database directory exists")?;
		let database_path = Self::get_db_path(path);
		let contents = if database_path.exists() {
			let file = File::open(&database_path).context("Failed to open database file")?;
			serde_json::from_reader(file).context("Failed to deserialize database contents")?
		} else {
			DatabaseContents::default()
		};

		let out = Self {
			dir: path.to_owned(),
			contents,
		};

		Ok(out)
	}

	/// Write the updated contents of the database handler to the database
	pub fn write(&self) -> anyhow::Result<()> {
		let path = Self::get_db_path(&self.dir);
		let file = File::create(path).context("Failed to create database file")?;
		serde_json::to_writer_pretty(file, &self.contents)
			.context("Failed to write database contents")?;

		Ok(())
	}

	/// Get the path to the main database file
	fn get_db_path(dir: &Path) -> PathBuf {
		dir.join("db.json")
	}

	/// Get whether an account in the database is still valid and logged in
	pub fn is_account_valid(&self, account_id: &str) -> bool {
		if let Some(account) = &self.contents.accounts.get(account_id) {
			let Ok(now) = utc_timestamp() else {
				return false;
			};

			now < account.expires
		} else {
			false
		}
	}

	/// Update an account
	pub fn update_account(
		&mut self,
		account: DatabaseAccount,
		account_id: &str,
	) -> anyhow::Result<()> {
		self.contents
			.accounts
			.insert(account_id.to_string(), account);

		self.write().context("Failed to write to database")?;
		Ok(())
	}

	/// Removes an account from the database
	pub fn remove_account(&mut self, account_id: &str) -> anyhow::Result<()> {
		self.contents.accounts.remove(account_id);

		self.write().context("Failed to write to database")?;
		Ok(())
	}

	/// Logs out an account from the database by removing their sensitive data, but not their passkey or account
	pub fn logout_account(&mut self, account_id: &str) -> anyhow::Result<()> {
		if let Some(account) = self.contents.accounts.get_mut(account_id) {
			account.sensitive = SensitiveAccountInfoSerialized::None;
		}

		Ok(())
	}

	/// Gets an account from the database, if it is present
	pub fn get_account(&self, account_id: &str) -> Option<&DatabaseAccount> {
		self.contents.accounts.get(account_id)
	}

	/// Gets an account mutably from the database, if it is present
	pub fn get_account_mut(&mut self, account_id: &str) -> Option<&mut DatabaseAccount> {
		self.contents.accounts.get_mut(account_id)
	}

	/// Gets an account, if it is present and valid
	pub fn get_valid_account(&self, account_id: &str) -> Option<&DatabaseAccount> {
		if self.is_account_valid(account_id) {
			self.get_account(account_id)
		} else {
			None
		}
	}

	/// Checks if any logged in accounts are present in the database
	pub fn has_logged_in_account(&self) -> bool {
		self.contents
			.accounts
			.values()
			.any(|x| x.sensitive != SensitiveAccountInfoSerialized::None)
	}
}

/// Structure for the auth database
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
struct DatabaseContents {
	/// The currently held accounts
	#[serde(alias = "users")]
	accounts: HashMap<String, DatabaseAccount>,
}

/// An account in the database
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseAccount {
	/// A unique ID for the account
	pub id: String,
	/// The username of the account
	pub username: String,
	/// The UUID of the account
	pub uuid: String,
	/// When the refresh token will expire, as a UTC timestamp in seconds
	pub expires: u64,
	/// Sensitive info for the account, serialized into a string and encoded
	/// using the public key
	pub sensitive: SensitiveAccountInfoSerialized,
	/// Passkey information for the account
	pub passkey: Option<PasskeyInfo>,
}

impl DatabaseAccount {
	/// Create a new database account with sensitive info
	pub fn new(
		id: String,
		username: String,
		uuid: String,
		expires: u64,
		sensitive: SensitiveAccountInfo,
	) -> anyhow::Result<Self> {
		let mut out = DatabaseAccount {
			id,
			username,
			uuid,
			expires,
			sensitive: SensitiveAccountInfoSerialized::Encrypted(Vec::new()),
			passkey: None,
		};
		out.set_sensitive_info(sensitive)
			.context("Failed to set sensitive information for account in database")?;
		Ok(out)
	}

	/// Checks if the account has a passkey
	pub fn has_passkey(&self) -> bool {
		self.passkey.is_some()
	}

	/// Checks if the account is logged in, where their sensitive info is present
	pub fn is_logged_in(&self) -> bool {
		!matches!(self.sensitive, SensitiveAccountInfoSerialized::None)
	}

	/// Get the account's private key from their passkey. Will fail if the passkey doesn't match
	/// and return none if the account doesn't have a passkey
	pub fn get_private_key(&self, passkey: &str) -> anyhow::Result<Option<RsaPrivateKey>> {
		if self.passkey.is_some() {
			let input_key = crate::passkey::generate_keys(passkey)
				.context("Failed to generate private key from input passkey")?;
			let expected_pub_key = self
				.get_public_key()
				.context("Failed to get stored public key")?
				.expect("Passkey info should be Some");
			ensure!(
				input_key.to_public_key() == expected_pub_key,
				"Passkey did not match"
			);
			Ok(Some(input_key))
		} else {
			Ok(None)
		}
	}

	/// Get the account's public key if they have one
	pub fn get_public_key(&self) -> anyhow::Result<Option<RsaPublicKey>> {
		if let Some(passkey_info) = &self.passkey {
			let key =
				hex::decode(&passkey_info.public_key).context("Failed to decode public key hex")?;
			let key = crate::passkey::recreate_public_key_bytes(&key)
				.context("Failed to recreate public key from stored data")?;
			Ok(Some(key))
		} else {
			Ok(None)
		}
	}

	/// Get the account's sensitive info if they don't have a passkey
	pub fn get_sensitive_info_no_passkey(&self) -> anyhow::Result<SensitiveAccountInfo> {
		ensure!(
			self.passkey.is_none(),
			"Account has a passkey that was not used"
		);
		let SensitiveAccountInfoSerialized::Raw(raw) = &self.sensitive else {
			bail!("Sensitive info is encrypted, not raw");
		};
		Ok(raw.clone())
	}

	/// Get the account's sensitive info using their private key
	pub fn get_sensitive_info_with_key(
		&self,
		private_key: &RsaPrivateKey,
	) -> anyhow::Result<SensitiveAccountInfo> {
		let SensitiveAccountInfoSerialized::Encrypted(encrypted) = &self.sensitive else {
			bail!("Sensitive account info is raw or empty");
		};
		let mut hex_decoded = Vec::new();
		for chunk in encrypted {
			let decoded = hex::decode(chunk)
				.context("Failed to deserialize hex of sensitive account info")?;
			hex_decoded.push(decoded);
		}
		let decoded = decrypt_chunks(&hex_decoded, private_key, Pkcs1v15Encrypt)
			.context("Failed to decrypt sensitive account info")?;
		let deserialized = serde_json::from_slice(&decoded)
			.context("Failed to deserialize sensitive account info")?;
		Ok(deserialized)
	}

	/// Set the account's sensitive info
	pub fn set_sensitive_info(&mut self, sensitive: SensitiveAccountInfo) -> anyhow::Result<()> {
		if self.has_passkey() {
			let public_key = self
				.get_public_key()
				.context("Failed to get account public key")?
				.expect("Account should have passkey");
			self.set_sensitive_info_impl(sensitive, &public_key)?;
		} else {
			self.sensitive = SensitiveAccountInfoSerialized::Raw(sensitive);
		}

		Ok(())
	}

	/// Implementation for setting the account's sensitive info with the given public key
	fn set_sensitive_info_impl(
		&mut self,
		sensitive: SensitiveAccountInfo,
		public_key: &RsaPublicKey,
	) -> anyhow::Result<()> {
		let serialized =
			serde_json::to_vec(&sensitive).context("Failed to serialize sensitive account info")?;
		let mut rng = rand::thread_rng();
		let encoded = encrypt_chunks(&serialized, public_key, &mut rng, Pkcs1v15Encrypt, 128)
			.context("Failed to encrypt sensitive account info")?;
		let mut hex_encoded = Vec::new();
		for chunk in encoded {
			let encoded = hex::encode(chunk);
			hex_encoded.push(encoded);
		}
		self.sensitive = SensitiveAccountInfoSerialized::Encrypted(hex_encoded);
		Ok(())
	}

	/// Set the account's passkey and update their sensitive information
	pub fn update_passkey(
		&mut self,
		old_passkey: Option<&str>,
		passkey: &str,
	) -> anyhow::Result<()> {
		let old_private_key = if let Some(old_passkey) = old_passkey {
			Some(
				crate::passkey::generate_keys(old_passkey)
					.context("Failed to generate private key from old passkey")?,
			)
		} else {
			None
		};

		let private_key = crate::passkey::generate_keys(passkey)
			.context("Failed to generate private key from new passkey")?;
		let pub_key = private_key.to_public_key();

		// Update sensitive info

		// Get the current sensitive info
		let sensitive = if self.has_passkey() {
			let Some(old_private_key) = old_private_key else {
				bail!("No old passkey provided to update sensitive account data");
			};
			self.get_sensitive_info_with_key(&old_private_key)
		} else {
			self.get_sensitive_info_no_passkey()
		}
		.context("Failed to get existing sensitive account data")?;
		self.set_sensitive_info_impl(sensitive, &pub_key)
			.context("Failed to set new sensitive account data")?;

		// We only update the passkey now just in case one of the above operations failed
		let n = pub_key.n().to_bytes_le();
		let n = hex::encode(n);
		self.passkey = Some(PasskeyInfo { public_key: n });

		Ok(())
	}
}

/// Sensitive info for an account that is encoded in a string
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SensitiveAccountInfo {
	/// The refresh token for the account
	pub refresh_token: Option<String>,
	/// The Xbox uid of the account, if applicable
	pub xbox_uid: Option<String>,
	/// The keypair of the account, if applicable
	pub keypair: Option<Keypair>,
	/// The Minecraft access token
	pub access_token: Option<String>,
	/// When the access token expires
	pub access_token_expires: Option<u64>,
}

/// Passkey information in the database
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PasskeyInfo {
	/// The public key that was derived from the passkey, as a hex string
	pub public_key: String,
}

/// Sensitive account data serialization format
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum SensitiveAccountInfoSerialized {
	/// No info
	None,
	/// Raw info with no passkey encryption
	Raw(SensitiveAccountInfo),
	/// Info encrypted with a passkey, as chunks of key-encoded hex strings
	Encrypted(Vec<String>),
}

/// Calculate the date to expire the refresh token at
pub fn calculate_expiration_date() -> u64 {
	let now = utc_timestamp().unwrap_or_default();
	now + REFRESH_TOKEN_EXPIRATION
}
