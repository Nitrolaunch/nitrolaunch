use std::fmt::Display;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
/// Configuration for an account
pub enum AccountConfig {
	/// Simple config with just the variant
	Simple(AccountVariant),
	/// Advanced config
	Advanced {
		/// The variant of the account
		#[serde(rename = "type")]
		variant: AccountVariant,
	},
}

/// Different variants of accounts for configuration
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AccountVariant {
	/// A Microsoft account
	Microsoft,
	/// A demo account
	Demo,
	/// An unknown account
	#[cfg_attr(not(feature = "schema"), serde(untagged))]
	Unknown(String),
}

impl Display for AccountVariant {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Microsoft {} => write!(f, "microsoft"),
			Self::Demo {} => write!(f, "demo"),
			Self::Unknown(other) => write!(f, "{other}"),
		}
	}
}
