#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;

use crate::util::is_valid_identifier;
use crate::versions::{parse_versioned_string, VersionPattern};

/// Type for the ID of a package
pub type PackageID = Arc<str>;

/// Used to store a request for a package that will be fulfilled later
#[derive(Debug, Clone, PartialOrd, Ord, Deserialize, Serialize)]
pub struct PkgRequest {
	/// The source of this request.
	/// Could be a dependent, a recommender, or anything else.
	pub source: PkgRequestSource,
	/// The ID of the package to request
	pub id: PackageID,
	/// The requested repository of the package
	pub repository: Option<String>,
	/// The requested content version of the package
	pub content_version: VersionPattern,
}

/// Where a package was requested from
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum PkgRequestSource {
	/// This package was required by the user
	UserRequire,
	/// This package was bundled by another package
	Bundled(ArcPkgReq),
	/// This package was depended on by another package
	Dependency(ArcPkgReq),
	/// This package was refused by another package
	Refused(ArcPkgReq),
	/// This package was requested by some automatic system
	Repository,
}

impl Ord for PkgRequestSource {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.to_num().cmp(&other.to_num())
	}
}

impl PartialOrd for PkgRequestSource {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl PkgRequestSource {
	/// Gets the source package of this package, if any
	pub fn get_source(&self) -> Option<ArcPkgReq> {
		match self {
			Self::Dependency(source) | Self::Bundled(source) => Some(source.clone()),
			_ => None,
		}
	}

	/// Gets whether this source list is only bundles that lead up to a UserRequire
	pub fn is_user_bundled(&self) -> bool {
		matches!(self, Self::Bundled(source) if source.source.is_user_bundled())
			|| matches!(self, Self::UserRequire)
	}

	/// Converts to a number, used for ordering
	fn to_num(&self) -> u8 {
		match self {
			Self::UserRequire => 0,
			Self::Bundled(..) => 1,
			Self::Dependency(..) => 2,
			Self::Refused(..) => 3,
			Self::Repository => 4,
		}
	}
}

impl PkgRequest {
	/// Create a new PkgRequest
	#[inline(always)]
	pub fn new(
		id: impl Into<PackageID>,
		source: PkgRequestSource,
		content_version: VersionPattern,
		repository: Option<String>,
	) -> Self {
		Self {
			id: id.into(),
			source,
			content_version,
			repository,
		}
	}

	/// Create a new PkgRequest that matches all content versions and repositories
	#[inline(always)]
	pub fn any(id: impl Into<PackageID>, source: PkgRequestSource) -> Self {
		Self::new(id, source, VersionPattern::Any, None)
	}

	/// Parse the package name and content version from a string
	pub fn parse(string: impl AsRef<str>, source: PkgRequestSource) -> Self {
		let string = string.as_ref();
		let (id_and_repo, version) = parse_versioned_string(string);

		let (id, repository) = if let Some(pos) = id_and_repo.find(":") {
			let id = &id_and_repo[pos + 1..];
			let repository = &id_and_repo[0..pos];
			// Empty repository should just be none
			(id, Some(repository).filter(|x| !x.is_empty()))
		} else {
			(id_and_repo, None)
		};
		Self {
			source,
			id: id.into(),
			content_version: version,
			repository: repository.map(|x| x.to_string()),
		}
	}

	/// Create a dependency list for debugging
	pub fn debug_sources(&self) -> String {
		self.debug_sources_inner(String::new())
	}

	/// Recursive inner function for debugging sources
	fn debug_sources_inner(&self, list: String) -> String {
		match &self.source {
			PkgRequestSource::UserRequire => format!("{}{list}", self.id),
			PkgRequestSource::Dependency(source) => {
				format!("{} -> {}", source.debug_sources_inner(list), self.id)
			}
			PkgRequestSource::Refused(source) => {
				format!("{} =X=> {}", source.debug_sources_inner(list), self.id)
			}
			PkgRequestSource::Bundled(bundler) => {
				format!("{} => {}", bundler.debug_sources_inner(list), self.id)
			}
			PkgRequestSource::Repository => format!("Repository -> {}{list}", self.id),
		}
	}
}

impl PartialEq for PkgRequest {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for PkgRequest {}

impl Hash for PkgRequest {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

impl Display for PkgRequest {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.id)
	}
}

/// A PkgRequest wrapped in an Arc
pub type ArcPkgReq = Arc<PkgRequest>;

/// Stability setting for a package
#[derive(Deserialize, Serialize, Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PackageStability {
	/// Whatever the latest stable version is
	#[default]
	Stable,
	/// Whatever the latest version is
	Latest,
}

impl PackageStability {
	/// Parse a PackageStability from a string
	pub fn parse_from_str(string: &str) -> Option<Self> {
		match string {
			"stable" => Some(Self::Stable),
			"latest" => Some(Self::Latest),
			_ => None,
		}
	}
}

/// The maximum length for a package identifier
pub const MAX_PACKAGE_ID_LENGTH: usize = 32;

/// Checks if a package identifier is valid
pub fn is_valid_package_id(id: &str) -> bool {
	if !is_valid_identifier(id) {
		return false;
	}

	for c in id.chars() {
		if c.is_ascii_uppercase() {
			return false;
		}
		if c == '_' || c == '.' {
			return false;
		}
	}

	if id.len() > MAX_PACKAGE_ID_LENGTH {
		return false;
	}

	true
}

/// Hashes used for package addons
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(default)]
pub struct PackageAddonHashes<T: Default> {
	/// The SHA-256 hash of this addon file
	pub sha256: T,
	/// The SHA-512 hash of this addon file
	pub sha512: T,
}

impl PackageAddonOptionalHashes {
	/// Checks if this set of optional hashes is empty
	pub fn is_empty(&self) -> bool {
		self.sha256.is_none() && self.sha512.is_none()
	}
}

/// Optional PackageAddonHashes
pub type PackageAddonOptionalHashes = PackageAddonHashes<Option<String>>;

/// Different types of packages, mostly AddonKinds
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
	/// A mod package
	Mod,
	/// A resource pack package
	ResourcePack,
	/// A datapack package
	Datapack,
	/// A plugin package
	Plugin,
	/// A shader package
	Shader,
	/// A package that bundles other packages
	Bundle,
}

/// Parameters for a package search
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PackageSearchParameters {
	/// The number of packages to return
	pub count: u8,
	/// How many results to skip
	pub skip: usize,
	/// The fuzzy search term for ids, names, or descriptions
	pub search: Option<String>,
	/// The addon kinds / package types to include
	pub types: Vec<PackageKind>,
	/// The Minecraft versions to include
	pub minecraft_versions: Vec<String>,
	/// The package categories to include
	pub categories: Vec<PackageCategory>,
}

/// Results for a package search
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PackageSearchResults {
	/// The package IDs returned by the search
	pub results: Vec<String>,
	/// The total number of results returned by the search, that weren't limited out
	pub total_results: usize,
}

/// A category for a package
#[allow(missing_docs)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PackageCategory {
	Adventure,
	Atmosphere,
	Audio,
	Blocks,
	Building,
	Cartoon,
	Challenge,
	Combat,
	Compatability,
	Decoration,
	Economy,
	Entities,
	Equipment,
	Exploration,
	Extensive,
	Fantasy,
	Fonts,
	Food,
	GameMechanics,
	Gui,
	Items,
	Language,
	Library,
	Lightweight,
	Magic,
	Minigame,
	Mobs,
	Multiplayer,
	Optimization,
	Realistic,
	Simplistic,
	Space,
	Social,
	Storage,
	Structures,
	Technology,
	Transportation,
	Tweaks,
	Utility,
	VanillaPlus,
	Worldgen,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_package_id_validation() {
		assert!(is_valid_package_id("hello"));
		assert!(is_valid_package_id("32"));
		assert!(is_valid_package_id("hello-world"));
		assert!(!is_valid_package_id("hello_world"));
		assert!(!is_valid_package_id("hello.world"));
		assert!(!is_valid_package_id("\\"));
		assert!(!is_valid_package_id(
			"very-very-long-long-long-package-name-thats-too-long"
		));
	}

	#[test]
	fn test_request_source_debug() {
		let req = PkgRequest::parse(
			"foo",
			PkgRequestSource::Dependency(Arc::new(PkgRequest::parse(
				"bar",
				PkgRequestSource::Dependency(Arc::new(PkgRequest::parse(
					"baz",
					PkgRequestSource::Repository,
				))),
			))),
		);
		let debug = req.debug_sources();
		assert_eq!(debug, "Repository -> baz -> bar -> foo");
	}

	#[test]
	fn test_pkg_req_parsing() {
		let req = PkgRequest::parse("foo", PkgRequestSource::UserRequire);
		assert_eq!(req.id, "foo".into());
		assert_eq!(req.repository, None);
		let req = PkgRequest::parse("foo@1.19.2", PkgRequestSource::UserRequire);
		assert_eq!(req.id, "foo".into());
		assert_eq!(req.content_version, VersionPattern::Single("1.19.2".into()));
		let req = PkgRequest::parse("modrinth:foo@1.19.2", PkgRequestSource::UserRequire);
		assert_eq!(req.id, "foo".into());
		assert_eq!(req.repository, Some("modrinth".into()));
		assert_eq!(req.content_version, VersionPattern::Single("1.19.2".into()));
		let req = PkgRequest::parse(":foo", PkgRequestSource::UserRequire);
		assert_eq!(req.id, "foo".into());
		assert_eq!(req.repository, None);

		let _ = PkgRequest::parse(":@", PkgRequestSource::UserRequire);
	}
}
