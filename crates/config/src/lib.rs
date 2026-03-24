#![warn(missing_docs)]

//! This crate contains serde structs for Nitrolaunch configuration. It does not provide
//! any functionality to actually read the config correctly, just to create it.

use std::{collections::HashMap, sync::Arc};

use account::AccountConfig;
use instance::InstanceConfig;
use nitro_shared::id::{InstanceID, TemplateID};
use preferences::PrefDeser;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use template::TemplateConfig;

/// Account configuration
pub mod account;
/// Instance configuration
pub mod instance;
/// Package configuration
pub mod package;
/// Global preferences configuration
pub mod preferences;
/// Template configuration
pub mod template;

/// Deserialization struct for user configuration
#[derive(Deserialize, Serialize, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(default)]
pub struct ConfigDeser {
	/// The list of configured accounts
	#[serde(alias = "users")]
	pub accounts: HashMap<String, AccountConfig>,
	/// The currently selected account
	#[serde(alias = "default_user")]
	pub default_account: Option<String>,
	/// The list of configured instances
	pub instances: HashMap<InstanceID, InstanceConfig>,
	/// The list of configured instance groups
	pub instance_groups: HashMap<Arc<str>, Vec<InstanceID>>,
	/// The list of configured templates
	#[serde(alias = "profiles")]
	pub templates: HashMap<TemplateID, TemplateConfig>,
	/// The base template
	#[serde(alias = "global_profile")]
	pub base_template: Option<TemplateConfig>,
	/// The global preferences
	pub preferences: PrefDeser,
}

/// Variants of instance-like config
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConfigKind {
	/// Instance config
	#[default]
	Instance,
	/// Template config
	Template,
	/// Base template config
	BaseTemplate,
}
