use nitro_shared::pkg::{PkgRequest, PkgRequestSource};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A list of overrides that apply to the package installation process
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(default)]
pub struct PackageOverrides {
	/// Packages to not install
	pub suppress: Vec<String>,
	/// Packages to force installation of
	pub force: Vec<String>,
}

/// Checks if a package is overridden in a list
pub fn is_package_overridden(package: &PkgRequest, list: &[String]) -> bool {
	list.iter()
		.map(|x| PkgRequest::parse(x, PkgRequestSource::UserRequire))
		.any(|x| &x == package)
}
