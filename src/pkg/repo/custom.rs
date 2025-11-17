use std::{collections::HashSet, sync::Arc};

use anyhow::Context;
use nitro_pkg::{repo::RepoMetadata, PackageSearchResults};
use nitro_plugin::{
	hook::call::HookHandle,
	hook::hooks::{
		PreloadPackages, PreloadPackagesArg, QueryCustomPackageRepository,
		QueryCustomPackageRepositoryArg, SearchCustomPackageRepository,
		SearchCustomPackageRepositoryArg, SyncCustomPackageRepository,
		SyncCustomPackageRepositoryArg,
	},
};
use nitro_shared::{
	output::NitroOutput,
	pkg::{ArcPkgReq, PackageSearchParameters},
};

use crate::{io::paths::Paths, pkg::PkgLocation, plugin::PluginManager};

use super::RepoQueryResult;

/// A custom package repository from a plugin
pub struct CustomPackageRepository {
	/// The ID of this repository
	id: String,
	/// The plugin that added this repository and implements all of its functions
	plugin: String,
	/// The metadata for the repository
	meta: RepoMetadata,
}

impl CustomPackageRepository {
	/// Creates a new CustomPackageRepository
	pub fn new(id: String, plugin: String, metadata: RepoMetadata) -> Self {
		Self {
			id,
			plugin,
			meta: metadata,
		}
	}

	/// Queries this repository for a package
	pub async fn query(
		&self,
		package: &str,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<RepoQueryResult>> {
		let arg = QueryCustomPackageRepositoryArg {
			repository: self.id.clone(),
			package: package.to_string(),
		};
		let result = plugins
			.call_hook_on_plugin(QueryCustomPackageRepository, &self.plugin, &arg, paths, o)
			.await
			.context("Failed to call query hook")?;

		let Some(result) = result else {
			return Ok(None);
		};

		let result = result.result(o).await?;

		Ok(result.map(|x| RepoQueryResult {
			location: PkgLocation::Inline(Arc::from(x.contents)),
			content_type: x.content_type,
			flags: x.flags,
		}))
	}

	/// Searches this repository for packages
	pub async fn search(
		&self,
		params: PackageSearchParameters,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<PackageSearchResults> {
		let arg = SearchCustomPackageRepositoryArg {
			repository: self.id.clone(),
			parameters: params,
		};
		let result = plugins
			.call_hook_on_plugin(SearchCustomPackageRepository, &self.plugin, &arg, paths, o)
			.await
			.context("Failed to call search hook")?;

		let Some(result) = result else {
			return Ok(PackageSearchResults::default());
		};

		result.result(o).await
	}

	/// Preloads multiple packages from this repository
	pub async fn preload(
		&self,
		packages: Vec<ArcPkgReq>,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let handle = self.get_preload_task(packages, plugins, paths, o).await?;
		let Some(handle) = handle else {
			return Ok(());
		};

		handle.result(o).await
	}

	/// Runs the preload hook on this repository and gives the HookHandle
	pub async fn get_preload_task(
		&self,
		packages: Vec<ArcPkgReq>,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Option<HookHandle<PreloadPackages>>> {
		// Deduplicate and remove packages not from this repo
		let packages: HashSet<_> = packages
			.into_iter()
			.filter_map(|x| {
				if x.repository.as_ref().is_some_and(|x| x != self.get_id()) {
					None
				} else {
					Some(x.id.to_string())
				}
			})
			.collect();

		if packages.is_empty() {
			return Ok(None);
		}

		let arg = PreloadPackagesArg {
			repository: self.id.clone(),
			packages: packages.into_iter().collect(),
		};

		plugins
			.call_hook_on_plugin(PreloadPackages, &self.plugin, &arg, paths, o)
			.await
			.context("Failed to call preload hook")
	}

	/// Syncs the cache for this repository
	pub async fn sync(
		&self,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let arg = SyncCustomPackageRepositoryArg {
			repository: self.id.clone(),
		};
		let result = plugins
			.call_hook_on_plugin(SyncCustomPackageRepository, &self.plugin, &arg, paths, o)
			.await
			.context("Failed to call sync hook")?;

		let Some(result) = result else {
			return Ok(());
		};

		result.result(o).await
	}

	/// Gets the ID for this repository
	pub fn get_id(&self) -> &str {
		&self.id
	}

	/// Gets the plugin ID for this repository
	pub fn get_plugin_id(&self) -> &str {
		&self.plugin
	}

	/// Gets the metadata for this repository
	pub fn get_meta(&self) -> &RepoMetadata {
		&self.meta
	}
}
