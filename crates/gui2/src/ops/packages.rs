use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use nitrolaunch::{
	instance_crate::{addon::Addon, lock::InstanceLockfile},
	pkg::search::{PackageMultiSearchResults, PackageSearchSession},
	pkg_crate::{metadata::PackageMetadata, properties::PackageProperties},
	shared::{
		id::InstanceID,
		output::{MessageContents, NitroOutput},
		pkg::{ArcPkgReq, PackageSearchParameters},
	},
};
use tokio::task::JoinSet;

use crate::prelude::*;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchInstanceLockfile {
	back_state: Captured<BackState>,
}

impl FetchInstanceLockfile {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl QueryCapability for FetchInstanceLockfile {
	type Ok = InstanceLockfile;
	type Err = anyhow::Error;
	type Keys = String;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let instance_id = keys.clone();

		query_spawn(async move {
			let config = back_state.config().await?;
			let instance = config
				.instances
				.get(&InstanceID::from(instance_id))
				.context("Instance does not exist")?;

			instance.get_lockfile(&back_state.paths)
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PreloadPackages {
	back_state: Captured<BackState>,
}

impl PreloadPackages {
	pub fn new(back_state: BackState) -> Mutation<Self> {
		Mutation::new(Self {
			back_state: Captured(back_state),
		})
	}
}

impl MutationCapability for PreloadPackages {
	type Ok = ();
	type Err = anyhow::Error;
	type Keys = Vec<ArcPkgReq>;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let packages = keys.clone();
		let back_state = self.back_state.clone();

		query_spawn(async move {
			if packages.is_empty() {
				return Ok(());
			}

			let config = back_state.config().await?;
			let mut o = back_state.output();

			o.debug(MessageContents::Simple(format!(
				"Preloading {} packages",
				packages.len()
			)));
			let out = config
				.packages
				.preload_packages(
					packages.iter(),
					&back_state.paths,
					&back_state.client,
					&mut back_state.output(),
				)
				.await?;
			o.debug(MessageContents::Simple("Packages preloaded".into()));

			Ok(out)
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchPackages {
	back_state: Captured<BackState>,
}

impl FetchPackages {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl QueryCapability for FetchPackages {
	type Ok = Arc<HashMap<ArcPkgReq, anyhow::Result<PkgInfo>>>;
	type Err = anyhow::Error;
	type Keys = Vec<ArcPkgReq>;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let packages = keys.clone();

		query_spawn(async move {
			if packages.is_empty() {
				return Ok(Arc::new(HashMap::new()));
			}

			let config = back_state.config().await?;
			let mut o = back_state.output();

			let paths = Arc::new(back_state.paths.clone());

			o.debug(MessageContents::Simple(format!(
				"Fetching {} packages",
				packages.len()
			)));
			let mut tasks = JoinSet::new();
			for req in packages {
				let reg = config.packages.clone();
				let paths = paths.clone();
				let client = back_state.client.clone();
				let mut o = o.get_greater_copy();
				tasks.spawn(async move {
					let package = match reg.get(&req, &paths, &client, &mut o).await {
						Ok(package) => package,
						Err(e) => return (req, Err(e)),
					};
					let meta = match package.get_metadata(&paths, &client).await {
						Ok(meta) => meta,
						Err(e) => return (req, Err(e)),
					};
					let props = match package.get_properties(&paths, &client).await {
						Ok(props) => props,
						Err(e) => return (req, Err(e)),
					};

					(req, Ok(PkgInfo { meta, props }))
				});
			}

			let mut out = HashMap::new();

			while let Some(task) = tasks.join_next().await {
				let (pkg, result) = task?;
				out.insert(pkg, result);
			}

			let out = Arc::new(out);
			o.debug(MessageContents::Simple("Packages Fetched".into()));

			Ok(out)
		})
	}
}

#[derive(Clone, Debug)]
pub struct PkgInfo {
	pub meta: Arc<PackageMetadata>,
	pub props: Arc<PackageProperties>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchInstanceAddons {
	back_state: Captured<BackState>,
}

impl FetchInstanceAddons {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl QueryCapability for FetchInstanceAddons {
	type Ok = Vec<Addon>;
	type Err = anyhow::Error;
	type Keys = String;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let instance_id = keys.clone();

		query_spawn(async move {
			let config = back_state.config().await?;
			let instance = config
				.instances
				.get(&InstanceID::from(instance_id))
				.context("Instance does not exist")?;

			instance.get_addons()
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SearchPackages {
	back_state: Captured<BackState>,
}

impl SearchPackages {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl QueryCapability for SearchPackages {
	type Ok = (PackageMultiSearchResults, PackageSearchSession);
	type Err = anyhow::Error;
	type Keys = SearchPackagesParams;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let mut params = keys.clone();

		query_spawn(async move {
			let config = back_state.config().await?;
			let mut o = back_state.output();

			let results = params
				.session
				.search(
					params.search,
					params.repo.as_deref(),
					config.packages,
					&back_state.paths,
					&back_state.client,
					&mut o,
				)
				.await?;
			Ok((results, params.session.0))
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SearchPackagesParams {
	pub search: PackageSearchParameters,
	pub session: Captured<PackageSearchSession>,
	pub repo: Option<String>,
}
