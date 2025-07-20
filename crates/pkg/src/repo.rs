use mcvm_shared::pkg::{PackageCategory, PackageKind};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

use crate::PackageContentType;

/// JSON format for a repository index
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RepoIndex {
	/// Metadata for the repository
	#[serde(default)]
	pub metadata: RepoMetadata,
	/// The packages available from the repository
	#[serde(default)]
	pub packages: HashMap<String, RepoPkgEntry>,
}

/// Metadata for a package repository
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(default)]
pub struct RepoMetadata {
	/// The display name of the repository
	pub name: Option<String>,
	/// The short description of the repository
	pub description: Option<String>,
	/// The MCVM version of the repository
	pub mcvm_version: Option<String>,
	/// A CSS color that represents the repository
	pub color: Option<String>,
	/// A CSS color for text that should contrast well with the main color
	pub text_color: Option<String>,
	/// The package types that this repository supports
	pub package_types: Vec<PackageKind>,
	/// The package categories that this repository supports
	pub package_categories: Vec<PackageCategory>,
}

/// An entry in the repository index package list that specifies information about the package
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct RepoPkgEntry {
	/// The URL to the package file
	#[serde(default)]
	pub url: Option<String>,
	/// The local or relative path to the package file
	#[serde(default)]
	pub path: Option<String>,
	/// Override for the content type of this package
	#[serde(default)]
	pub content_type: Option<PackageContentType>,
	/// Flags for this package
	#[serde(default)]
	pub flags: HashSet<PackageFlag>,
}

/// Flags that can be applied to packages by repositories to provide information about them
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PackageFlag {
	/// The package file has not been updated to reflect the newest versions of the content
	OutOfDate,
	/// This package has been deprecated in favor of another one
	Deprecated,
	/// This package has security or safety vulnerabilities
	Insecure,
	/// The package provides malicious content
	Malicious,
}

/// Get the URL of the repository api
pub fn get_api_url(base_url: &str) -> String {
	// Remove trailing slash
	let base_url = if let Some(stripped) = base_url.strip_suffix('/') {
		stripped
	} else {
		base_url
	};

	base_url.to_string() + "/api/mcvm/"
}

/// Get the URL of the repository index file
pub fn get_index_url(base_url: &str) -> String {
	let api_url = get_api_url(base_url);

	api_url + "index.json"
}
