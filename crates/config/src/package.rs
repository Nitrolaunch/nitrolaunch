use std::borrow::Cow;
use std::fmt::Display;

use anyhow::bail;
use nitro_shared::pkg::{PackageStability, PkgRequest, PkgRequestSource, is_valid_package_id};
use nitro_shared::util::{DefaultExt, is_valid_identifier};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Different representations for the configuration of a package in deserialization
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum PackageConfigDeser {
	/// Basic configuration for a repository package with just the package request
	Basic(String),
	/// Full configuration for a package
	Full(FullPackageConfig),
}

/// Full configuration for a package
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FullPackageConfig {
	/// The ID / request of the pcakage
	#[serde(alias = "req")]
	pub id: String,
	/// The package's enabled features
	#[serde(default)]
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub features: Vec<String>,
	/// Whether or not to use the package's default features
	#[serde(default = "use_default_features_default")]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub use_default_features: bool,
	/// Permissions for the package
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub permissions: EvalPermissions,
	/// Expected stability for the package
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stability: Option<PackageStability>,
	/// Worlds to use for the package
	#[serde(default)]
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub worlds: Vec<String>,
	/// Desired content version for this package
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content_version: Option<String>,
	/// Whether this package is optional
	#[serde(default)]
	#[serde(skip_serializing_if = "DefaultExt::is_default")]
	pub optional: bool,
}

/// Trick enum used to make deserialization work in the way we want
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
	/// Yeah this is kinda stupid
	Local,
}

/// Default value for use_default_features
fn use_default_features_default() -> bool {
	true
}

impl Display for PackageConfigDeser {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Basic(id) => id,
				Self::Full(FullPackageConfig { id, .. }) => id,
			}
		)
	}
}

impl PackageConfigDeser {
	/// Get the package request of the config as a string
	pub fn get_id(&self) -> &str {
		match &self {
			Self::Basic(req) => req,
			Self::Full(cfg) => &cfg.id,
		}
	}

	/// Get the package request of the config
	pub fn get_req(&self) -> PkgRequest {
		PkgRequest::parse(self.get_id(), PkgRequestSource::UserRequire)
	}

	/// Sets the package request of the config
	pub fn set_id(&mut self, new_id: String) {
		match self {
			Self::Basic(req) => *req = new_id,
			Self::Full(cfg) => cfg.id = new_id,
		}
	}

	/// Get the features of the config
	pub fn get_features(&self) -> Vec<String> {
		match &self {
			Self::Basic(..) => Vec::new(),
			Self::Full(cfg) => cfg.features.clone(),
		}
	}

	/// Get the use_default_features option of the config
	pub fn get_use_default_features(&self) -> bool {
		match &self {
			Self::Basic(..) => use_default_features_default(),
			Self::Full(cfg) => cfg.use_default_features,
		}
	}

	/// Get the permissions of the config
	pub fn get_permissions(&self) -> EvalPermissions {
		match &self {
			Self::Basic(..) => EvalPermissions::Standard,
			Self::Full(cfg) => cfg.permissions,
		}
	}

	/// Get the stability of the config
	pub fn get_stability(&self, default_stability: PackageStability) -> PackageStability {
		match &self {
			Self::Basic(..) => default_stability,
			Self::Full(cfg) => cfg.stability.unwrap_or(default_stability),
		}
	}

	/// Get the worlds of the config
	pub fn get_worlds(&'_ self) -> Cow<'_, [String]> {
		match &self {
			Self::Basic(..) => Cow::Owned(Vec::new()),
			Self::Full(cfg) => Cow::Borrowed(&cfg.worlds),
		}
	}

	/// Get the content version of the config
	pub fn get_content_version(&self) -> Option<&String> {
		match &self {
			Self::Basic(..) => None,
			Self::Full(cfg) => cfg.content_version.as_ref(),
		}
	}

	/// Get the optional setting of the config
	pub fn get_optional(&self) -> bool {
		match &self {
			Self::Basic(..) => false,
			Self::Full(cfg) => cfg.optional,
		}
	}

	/// Validate this config
	pub fn validate(&self) -> anyhow::Result<()> {
		let req = self.get_req();
		if !is_valid_package_id(&req.id) {
			bail!("Invalid package ID '{req}'");
		}

		for feature in self.get_features() {
			if !is_valid_identifier(&feature) {
				bail!("Invalid package feature string '{feature}'");
			}
		}

		Ok(())
	}
}

/// Adds or changes a package in a package configuration list.
/// Changes only the request, and leaves the other fields as they are, unless this is a new package.
pub fn add_or_update_package_config(
	configs: &mut Vec<PackageConfigDeser>,
	new_config: PackageConfigDeser,
) {
	for pkg in configs.iter_mut() {
		if pkg.get_req() == new_config.get_req() {
			pkg.set_id(new_config.get_id().to_string());
			return;
		}
	}
	configs.push(new_config);
}

/// Permissions level for an evaluation
#[derive(Deserialize, Serialize, Debug, Copy, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum EvalPermissions {
	/// Restricts certain operations that would normally be allowed
	Restricted,
	/// Standard permissions. Allow all common operations
	#[default]
	Standard,
	/// Allow execution of things that could compromise security
	Elevated,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_add_empty() {
		let mut configs = Vec::new();
		let new_config = PackageConfigDeser::Basic("pkg1".to_string());
		add_or_update_package_config(&mut configs, new_config);
		assert_eq!(configs.len(), 1);
	}

	#[test]
	fn test_add_update() {
		let mut configs = vec![PackageConfigDeser::Basic("pkg1".to_string())];
		let new_config = PackageConfigDeser::Basic("pkg1".to_string());
		add_or_update_package_config(&mut configs, new_config);
		assert_eq!(configs.len(), 1);
	}

	#[test]
	fn test_add_new() {
		let mut configs = vec![PackageConfigDeser::Basic("pkg1".to_string())];
		let new_config = PackageConfigDeser::Basic("pkg2".to_string());
		add_or_update_package_config(&mut configs, new_config);
		assert_eq!(configs.len(), 2);
	}
}
