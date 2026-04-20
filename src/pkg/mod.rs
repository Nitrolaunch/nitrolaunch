/// Core packages that are built into the binary
mod core;
/// Package evaluation functions
pub mod eval;
/// Registry used to store packages
pub mod reg;
/// Interacting with package repositories
pub mod repo;

use crate::io::paths::Paths;
use nitro_core::net::download;
use nitro_pkg::declarative::{
	deserialize_declarative_package, DeclarativeAddonVersion, DeclarativeConditionSet,
	DeclarativePackage,
};
use nitro_pkg::repo::PackageFlag;
use nitro_pkg::PackageContentType;
use nitro_shared::try_3;
use nitro_shared::util::DeserListOrSingle;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use self::core::get_core_package;
use anyhow::{anyhow, bail, Context};
use nitro_parse::parse::{lex_and_parse, Parsed};
use nitro_pkg::metadata::{eval_metadata, PackageMetadata};
use nitro_pkg::properties::{eval_properties, PackageProperties};
use nitro_shared::pkg::ArcPkgReq;
use reqwest::Client;

/// An installable package that loads content into your game
#[derive(Debug)]
pub struct Package {
	/// The package request
	pub req: ArcPkgReq,
	/// Where the package is being retrieved from
	pub location: PkgLocation,
	/// Type of the content in the package
	pub content_type: PackageContentType,
	/// Flags for the package from the repository
	pub flags: HashSet<PackageFlag>,
	text: OnceLock<Arc<str>>,
	contents: OnceLock<PkgContents>,
	metadata: OnceLock<Arc<PackageMetadata>>,
	properties: OnceLock<Arc<PackageProperties>>,
}

/// Location of a package
#[derive(Debug, Clone)]
pub enum PkgLocation {
	/// Contained on the local filesystem
	Local(PathBuf),
	/// Contained on an external repository
	Remote {
		/// The URL of the remote package
		url: Option<String>,
		/// The ID of the repository this package is from
		repo_id: String,
	},
	/// Contents are included inline in the PkgLocation
	Inline(Arc<str>),
	/// Included in the binary
	Core,
}

/// Type of data inside a package
#[derive(Debug)]
pub enum PkgContents {
	/// A package script
	Script(Parsed),
	/// A declarative package
	Declarative(Box<DeclarativePackage>),
}

impl PkgContents {
	/// Get the contents with an assertion that it is a script package
	pub fn get_script_contents(&self) -> &Parsed {
		if let Self::Script(parsed) = &self {
			parsed
		} else {
			panic!("Attempted to get script package contents from a non-script package");
		}
	}

	/// Get the contents with an assertion that it is a declarative package
	pub fn get_declarative_contents(&self) -> &DeclarativePackage {
		if let Self::Declarative(contents) = &self {
			contents
		} else {
			panic!("Attempted to get declarative package contents from a non-declarative package");
		}
	}

	/// Get the contents of a script package
	pub fn get_script_contents_optional(&self) -> Option<&Parsed> {
		if let Self::Script(parsed) = &self {
			Some(parsed)
		} else {
			None
		}
	}

	/// Get the contents of a declarative package
	pub fn get_declarative_contents_optional(&self) -> Option<&DeclarativePackage> {
		if let Self::Declarative(contents) = &self {
			Some(contents)
		} else {
			None
		}
	}
}

impl Package {
	/// Create a new Package
	pub fn new(
		req: ArcPkgReq,
		location: PkgLocation,
		content_type: PackageContentType,
		flags: HashSet<PackageFlag>,
	) -> Self {
		Self {
			req,
			location,
			content_type,
			flags,
			text: OnceLock::new(),
			contents: OnceLock::new(),
			metadata: OnceLock::new(),
			properties: OnceLock::new(),
		}
	}

	/// Get the cached file name of the package
	pub fn filename(&self) -> String {
		let extension = match self.content_type {
			PackageContentType::Declarative => ".json",
			PackageContentType::Script => ".pkg.txt",
		};
		format!("{}{extension}", self.req.id)
	}

	/// Get the cached path of the package
	pub fn cached_path(&self, paths: &Paths) -> PathBuf {
		paths.pkg_cache.join(self.filename())
	}

	/// Remove the cached package file
	pub fn remove_cached(&self, paths: &Paths) -> anyhow::Result<()> {
		let path = self.cached_path(paths);
		if path.exists() {
			fs::remove_file(path)?;
		}
		Ok(())
	}

	/// Ensure the raw contents of the package
	pub async fn ensure_loaded(
		&self,
		paths: &Paths,
		force: bool,
		client: &Client,
	) -> anyhow::Result<()> {
		if self.text.get().is_some() {
			return Ok(());
		}

		match &self.location {
			PkgLocation::Local(path) => {
				if !path.exists() {
					bail!("Local package path does not exist");
				}
				let _ = self
					.text
					.set(Arc::from(tokio::fs::read_to_string(path).await?));
			}
			PkgLocation::Remote { url, .. } => {
				let path = self.cached_path(paths);
				if !force && path.exists() {
					let _ = self
						.text
						.set(Arc::from(tokio::fs::read_to_string(path).await?));
				} else {
					let url = url.as_ref().expect("URL for remote package missing");
					let text = try_3!({ download::text(url, client).await })?;
					tokio::fs::write(&path, &text).await?;
					let _ = self.text.set(Arc::from(text));
				}
			}
			PkgLocation::Core => {
				let contents = get_core_package(&self.req.id)
					.ok_or(anyhow!("Package is not a core package"))?;
				let _ = self.text.set(Arc::from(contents));
			}
			PkgLocation::Inline(contents) => {
				let _ = self.text.set(contents.clone());
			}
		};

		Ok(())
	}

	/// Returns a task that download's the package file if necessary. This will not
	/// update the contents and is only useful when doing repo resyncs
	pub fn get_download_task(
		&self,
		paths: &Paths,
		force: bool,
		client: &Client,
	) -> Option<impl Future<Output = anyhow::Result<()>> + 'static> {
		if let PkgLocation::Remote { url, .. } = &self.location {
			let path = self.cached_path(paths);
			if force || !path.exists() {
				let url = url
					.as_ref()
					.expect("URL for remote package missing")
					.clone();
				let client = client.clone();
				return Some(async move { try_3!({ download::file(&url, &path, &client).await }) });
			}
		}

		None
	}

	/// Parse the contents of the package
	pub async fn parse<'this>(
		&'this self,
		paths: &Paths,
		client: &Client,
	) -> anyhow::Result<&'this PkgContents> {
		self.ensure_loaded(paths, false, client).await?;
		if let Some(contents) = self.contents.get() {
			return Ok(contents);
		}

		let text = self.text.get().unwrap();

		match self.content_type {
			PackageContentType::Script => {
				let parsed = lex_and_parse(&text)?;
				let _ = self.contents.set(PkgContents::Script(parsed));
			}
			PackageContentType::Declarative => {
				let contents = deserialize_declarative_package(&text)
					.context("Failed to deserialize declarative package")?;
				let _ = self
					.contents
					.set(PkgContents::Declarative(Box::new(contents)));
			}
		}

		Ok(self.contents.get().unwrap())
	}

	/// Get the metadata of the package
	pub async fn get_metadata<'a>(
		&'a self,
		paths: &Paths,
		client: &Client,
	) -> anyhow::Result<Arc<PackageMetadata>> {
		self.parse(paths, client).await.context("Failed to parse")?;

		if let Some(metadata) = self.metadata.get() {
			return Ok(metadata.clone());
		}

		match self.content_type {
			PackageContentType::Script => {
				let contents = self.contents.get().unwrap();
				let PkgContents::Script(parsed) = contents else {
					bail!("Content type does not match");
				};
				let metadata = eval_metadata(parsed).context("Failed to evaluate metadata")?;
				let _ = self.metadata.set(Arc::new(metadata));
			}
			PackageContentType::Declarative => {
				let contents = self.contents.get().unwrap();
				let PkgContents::Declarative(declarative) = contents else {
					bail!("Content type does not match");
				};

				let _ = self.metadata.set(Arc::new(declarative.meta.clone()));
			}
		}

		Ok(self.metadata.get().unwrap().clone())
	}

	/// Get the properties of the package
	pub async fn get_properties<'a>(
		&'a self,
		paths: &Paths,
		client: &Client,
	) -> anyhow::Result<Arc<PackageProperties>> {
		self.parse(paths, client).await.context("Failed to parse")?;

		if let Some(properties) = self.properties.get() {
			return Ok(properties.clone());
		}

		match self.content_type {
			PackageContentType::Script => {
				let contents = self.contents.get().unwrap();
				let PkgContents::Script(parsed) = contents else {
					bail!("Content type does not match");
				};
				if self.properties.get().is_none() {
					let properties =
						eval_properties(parsed).context("Failed to evaluate properties")?;
					let _ = self.properties.set(Arc::new(properties));
				}
			}
			PackageContentType::Declarative => {
				let contents = self.contents.get().unwrap();
				let PkgContents::Declarative(declarative) = contents else {
					bail!("Content type does not match");
				};

				let _ = self
					.properties
					.set(Arc::new(declarative.properties.clone()));
			}
		}

		Ok(self.properties.get().unwrap().clone())
	}

	/// Get the declarative contents of the package
	pub async fn get_declarative_contents<'a>(
		&'a self,
		paths: &Paths,
		client: &Client,
	) -> anyhow::Result<Option<&'a DeclarativePackage>> {
		self.parse(paths, client).await.context("Failed to parse")?;

		let contents = self.contents.get().unwrap();
		if let PkgContents::Declarative(contents) = contents {
			Ok(Some(&contents))
		} else {
			Ok(None)
		}
	}

	/// Gets the content versions of this addon
	pub async fn get_content_versions(
		&self,
		paths: &Paths,
		client: &Client,
	) -> anyhow::Result<Vec<Cow<'_, DeclarativeAddonVersion>>> {
		let contents = self.get_declarative_contents(paths, client).await?;
		let Some(contents) = contents else {
			let properties = self.get_properties(paths, client).await?;

			return Ok(properties
				.content_versions
				.iter()
				.flatten()
				.map(|x| {
					Cow::Owned(DeclarativeAddonVersion {
						conditional_properties: DeclarativeConditionSet {
							content_versions: Some(DeserListOrSingle::Single(x.clone())),
							..Default::default()
						},
						..Default::default()
					})
				})
				.collect());
		};

		// Combine the same content version across multiple addons into a single version if possible
		let mut versions_with_ids = HashMap::new();
		let mut versions_without_ids = Vec::new();

		for addon in contents.addons.values() {
			for version in &addon.versions {
				let content_version =
					if let Some(versions) = &version.conditional_properties.content_versions {
						versions.first()
					} else {
						None
					};

				if let Some(content_version) = content_version {
					if !versions_with_ids.contains_key(content_version) {
						versions_with_ids.insert(content_version.clone(), version);
					}
				} else {
					versions_without_ids.push(version);
				}
			}
		}

		Ok(versions_without_ids
			.into_iter()
			.chain(versions_with_ids.into_values())
			.map(Cow::Borrowed)
			.collect())
	}

	/// Get the text contents of the package
	pub async fn get_text<'a>(
		&'a self,
		paths: &Paths,
		client: &Client,
	) -> anyhow::Result<Arc<str>> {
		self.ensure_loaded(paths, false, client)
			.await
			.context("Failed to parse")?;

		Ok(self.text.get().unwrap().clone())
	}
}

#[cfg(test)]
mod tests {
	use nitro_pkg::PkgRequest;

	use super::*;

	#[test]
	fn test_package_id() {
		let package = Package::new(
			PkgRequest::parse("sodium", nitro_pkg::PkgRequestSource::UserRequire).arc(),
			PkgLocation::Remote {
				url: None,
				repo_id: String::new(),
			},
			PackageContentType::Script,
			HashSet::new(),
		);
		assert_eq!(package.filename(), "sodium.pkg.txt".to_string());

		let package = Package::new(
			PkgRequest::parse("fabriclike-api", nitro_pkg::PkgRequestSource::UserRequire).arc(),
			PkgLocation::Remote {
				url: None,
				repo_id: String::new(),
			},
			PackageContentType::Declarative,
			HashSet::new(),
		);
		assert_eq!(package.filename(), "fabriclike-api.json".to_string());
	}
}
