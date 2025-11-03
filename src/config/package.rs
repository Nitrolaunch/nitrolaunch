use std::sync::Arc;

use anyhow::ensure;
use nitro_config::package::{EvalPermissions, PackageConfigDeser};
use nitro_pkg::properties::PackageProperties;
use nitro_shared::pkg::{ArcPkgReq, PackageID, PackageStability};

use nitro_pkg::{PkgRequest, PkgRequestSource};

/// Stored configuration for a package
#[derive(Clone, Debug)]
pub struct PackageConfig {
	/// The ID of the package
	pub id: PackageID,
	/// The package's enabled features
	pub features: Vec<String>,
	/// Whether or not to use the package's default features
	pub use_default_features: bool,
	/// Permissions for the package
	pub permissions: EvalPermissions,
	/// Expected stability for the package
	pub stability: PackageStability,
	/// Worlds to use for the package
	pub worlds: Vec<String>,
	/// Desired content version for this package
	pub content_version: Option<String>,
	/// Whether this package is optional
	pub optional: bool,
}

/// Where a package was configured from
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageConfigSource {
	/// Configured for one template
	Template,
	/// Configured for one instance
	Instance,
}

impl PackageConfig {
	/// Create the default configuration for a package with a package ID
	pub fn from_id(id: PackageID) -> Self {
		Self {
			id,
			features: Vec::new(),
			use_default_features: true,
			permissions: EvalPermissions::default(),
			stability: PackageStability::default(),
			worlds: Vec::new(),
			content_version: None,
			optional: false,
		}
	}

	/// Calculate the features of the config
	pub fn calculate_features(
		&self,
		properties: &PackageProperties,
	) -> anyhow::Result<Vec<String>> {
		let empty = Vec::new();
		let allowed_features = properties.features.as_ref().unwrap_or(&empty);

		for feature in &self.features {
			ensure!(
				allowed_features.contains(feature),
				"Configured feature '{feature}' does not exist"
			);
		}

		let mut out = self.features.clone();
		if self.use_default_features {
			let default_features = properties.default_features.clone().unwrap_or_default();
			out.extend(default_features);
		}

		Ok(out)
	}

	/// Get the request of the config
	pub fn get_request(&self) -> ArcPkgReq {
		Arc::new(PkgRequest::parse(
			self.id.clone(),
			PkgRequestSource::UserRequire,
		))
	}
}

/// Reads configuration for a package
pub fn read_package_config(
	config: PackageConfigDeser,
	template_stability: PackageStability,
	// source: PackageConfigSource,
) -> PackageConfig {
	let id = config.get_pkg_id();

	PackageConfig {
		id,
		features: config.get_features(),
		use_default_features: config.get_use_default_features(),
		permissions: config.get_permissions(),
		stability: config.get_stability(template_stability),
		worlds: config.get_worlds().into_owned(),
		content_version: None,
		optional: config.get_optional(),
	}
}
