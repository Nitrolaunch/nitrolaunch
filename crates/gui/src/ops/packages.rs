use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use nitrolaunch::{
	config::modifications::{ConfigModification, apply_modifications_and_write},
	config_crate::{ConfigKind, package::PackageConfigDeser},
	instance::{Instance, update::manager::UpdateSettings},
	instance_crate::{addon::Addon, lock::InstanceLockfile},
	pkg::search::{PackageMultiSearchResults, PackageSearchSession},
	pkg_crate::{
		PackageMetaAndProps, declarative::DeclarativeAddonVersion, metadata::PackageMetadata,
		properties::PackageProperties,
	},
	shared::{
		Side, UpdateDepth,
		id::{InstanceID, TemplateID},
		output::{MessageContents, NitroOutput, NoOp},
		pkg::{ArcPkgReq, PackageSearchParameters},
	},
};
use tokio::task::JoinSet;

use crate::{
	dependency::BackDependency, ops::task::Task, pages::config::ConfiguredItem, prelude::*,
	simple_query,
};

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

		query_spawn(back_state.0.clone(), async move {
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

		query_spawn(back_state.0.clone(), async move {
			if packages.is_empty() {
				return Ok(());
			}

			let config = back_state.config().await?;
			let mut o = back_state.output();

			o.debug(MessageContents::Simple(format!(
				"Preloading {} packages",
				packages.len()
			)));
			config
				.packages
				.preload_packages(
					packages.iter(),
					&back_state.paths,
					&back_state.client,
					&mut back_state.output(),
				)
				.await?;
			o.debug(MessageContents::Simple("Packages preloaded".into()));

			Ok(())
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

		query_spawn(back_state.0.clone(), async move {
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

		query_spawn(back_state.0.clone(), async move {
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

		query_spawn(back_state.0.clone(), async move {
			let config = back_state.config().await?;
			let mut o = back_state.output();
			o.set_task(Task::SearchPackages);

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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchPackageDetails {
	back_state: Captured<BackState>,
}

impl FetchPackageDetails {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl QueryCapability for FetchPackageDetails {
	type Ok = PackageMetaAndProps;
	type Err = anyhow::Error;
	type Keys = ArcPkgReq;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let req = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let config = back_state.config().await?;

			let mut o = back_state.output();
			let package = config
				.packages
				.get(&req, &back_state.paths, &back_state.client, &mut o)
				.await?;
			let meta = package
				.get_metadata(&back_state.paths, &back_state.client)
				.await?;
			let props = package
				.get_properties(&back_state.paths, &back_state.client)
				.await?;

			Ok(PackageMetaAndProps { meta, props })
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchPackageContentVersions {
	back_state: Captured<BackState>,
}

impl FetchPackageContentVersions {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl QueryCapability for FetchPackageContentVersions {
	type Ok = Vec<DeclarativeAddonVersion>;
	type Err = anyhow::Error;
	type Keys = ArcPkgReq;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let req = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let config = back_state.config().await?;

			let mut o = back_state.output();
			let package = config
				.packages
				.get(&req, &back_state.paths, &back_state.client, &mut o)
				.await?;
			let versions = package
				.get_content_versions(&back_state.paths, &back_state.client)
				.await?;

			Ok(versions.into_iter().map(|x| x.into_owned()).collect())
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InstallPackage {
	back_state: Captured<BackState>,
}

impl InstallPackage {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl MutationCapability for InstallPackage {
	type Ok = ();
	type Err = anyhow::Error;
	type Keys = (ArcPkgReq, PackageInstallLocation);

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let config = back_state.config().await?;
			let mut raw_config = back_state.raw_config().await?;
			let mut o = back_state.output();

			let modification = match keys.1 {
				PackageInstallLocation::Instance(instance_id) => {
					let instance = config
						.instances
						.get(&instance_id)
						.context("Instance does not exist")?;

					let mut inst_config = instance.original_config().clone();
					inst_config
						.packages
						.push(PackageConfigDeser::Basic(keys.0.to_string().into()));

					ConfigModification::UpdateInstance(instance_id.clone(), inst_config)
				}
				PackageInstallLocation::Template(template_id, side) => {
					let template = config
						.templates
						.get(&template_id)
						.context("Template does not exist")?;
					let mut template = template.clone();

					let pkg = PackageConfigDeser::Basic(keys.0.to_string().into());
					template.packages.add_package(pkg, side);

					ConfigModification::UpdateTemplate(template_id.clone(), template)
				}
				PackageInstallLocation::BaseTemplate(side) => {
					let mut template = raw_config.base_template.clone().unwrap_or_default();

					let pkg = PackageConfigDeser::Basic(keys.0.to_string().into());
					template.packages.add_package(pkg, side);

					raw_config.base_template = Some(template);
					apply_modifications_and_write(
						&mut raw_config,
						Vec::new(),
						&back_state.paths,
						&back_state.plugins,
						&mut o,
					)
					.await?;
					return Ok(());
				}
				PackageInstallLocation::InstanceModpack(instance_id) => {
					let instance = config
						.instances
						.get(&instance_id)
						.context("Instance does not exist")?;

					let mut inst_config = instance.original_config().clone();
					inst_config.modpack = Some(keys.0.to_string());

					ConfigModification::UpdateInstance(instance_id.clone(), inst_config)
				}
				PackageInstallLocation::TemplateModpack(template_id) => {
					let template = config
						.templates
						.get(&template_id)
						.context("Template does not exist")?;
					let mut template = template.clone();
					template.instance.modpack = Some(keys.0.to_string());

					ConfigModification::UpdateTemplate(template_id.clone(), template)
				}
				PackageInstallLocation::NewInstanceModpack(instance_id) => {
					o.set_task(Task::InstallModpack);

					let core = config
						.get_core(
							None,
							&UpdateSettings {
								depth: UpdateDepth::Shallow,
								offline_auth: true,
							},
							&back_state.client,
							&config.plugins,
							&back_state.paths,
							&mut NoOp,
						)
						.await?;

					let version_manifest = core
						.get_version_manifest(None, UpdateDepth::Full, &mut NoOp)
						.await?;

					let config = Instance::create_from_modpack_package(
						&instance_id,
						&keys.0,
						Side::Client,
						version_manifest.list.clone(),
						&config.packages,
						&config.plugins,
						&back_state.client,
						&back_state.paths,
						&mut o,
					)
					.await
					.context("Failed to import the new instance")?;

					ConfigModification::AddInstance(instance_id, config)
				}
			};

			apply_modifications_and_write(
				&mut raw_config,
				vec![modification],
				&back_state.paths,
				&back_state.plugins,
				&mut o,
			)
			.await?;

			back_state.invalidate(BackDependency::Items);

			Ok(())
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PackageInstallLocation {
	Instance(InstanceID),
	Template(TemplateID, Option<Side>),
	BaseTemplate(Option<Side>),
	InstanceModpack(InstanceID),
	TemplateModpack(TemplateID),
	NewInstanceModpack(InstanceID),
}

simple_query!(
	name = CheckPackageCompatability,
	ok = Option<PackageCompatabilityError>,
	err = anyhow::Error,
	keys = CheckPackageCompatabilityKeys,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let mut o = back_state.output();
			let config = back_state.config().await?;

			let package = config
				.packages
				.get(&keys.package, &back_state.paths, &back_state.client, &mut o)
				.await?;
			let props = package
				.get_properties(&back_state.paths, &back_state.client)
				.await?;

			let default_mc_versions = Vec::new();
			let mc_versions = props
				.supported_versions
				.as_ref()
				.unwrap_or(&default_mc_versions);

			let default_loaders = Vec::new();
			let loaders = props
				.supported_loaders
				.as_ref()
				.unwrap_or(&default_loaders);

			let manifest = back_state.versions().await?;

			match keys.item.ty {
				ConfigKind::Instance => {
					let instance = config
						.instances
						.get(&InstanceID::from(keys.item.id.unwrap()))
						.context("Instance does not exist")?;

					let mc_version = back_state
						.canonicalize_version(
							Some(instance.id()),
							ConfigKind::Instance,
							&instance.version().clone().to_serialized(),
						)
						.await
						.context("Failed to get instance version")?;

					if !mc_versions.is_empty()
						&& !mc_versions
							.iter()
							.any(|x| x.matches_single(&mc_version, &manifest.list))
					{
						return Ok(Some(PackageCompatabilityError::WrongMinecraftVersion));
					}

					if !loaders.is_empty() && !loaders.iter().any(|x| x.matches(instance.loader())) {
						return Ok(Some(PackageCompatabilityError::WrongLoader));
					}
				}
				ConfigKind::Template | ConfigKind::BaseTemplate => {
					let id = keys.item.id;
					let template = match keys.item.ty {
						ConfigKind::Template => config
							.consolidated_templates
							.get(&TemplateID::from(id.clone().unwrap()))
							.context("Template does not exist")?,
						ConfigKind::BaseTemplate => &config.base_template,
						_ => unreachable!(),
					};

					if let Some(mc_version) = &template.instance.version {
						let mc_version = back_state
							.canonicalize_version(id.as_deref(), ConfigKind::Template, mc_version)
							.await
							.context("Failed to get template version")?;
						if !mc_versions.is_empty()
							&& !mc_versions
								.iter()
								.any(|x| x.matches_single(&mc_version, &manifest.list))
						{
							return Ok(Some(PackageCompatabilityError::WrongMinecraftVersion));
						}
					}

					let client_loader = template.client_loader().map(|x| x.0).unwrap_or_default();
					let server_loader = template.server_loader().map(|x| x.0).unwrap_or_default();
					if !loaders.is_empty() && !loaders.iter().any(|x| x.matches(&client_loader) || x.matches(&server_loader)) {
						return Ok(Some(PackageCompatabilityError::WrongLoader));
					}
				}
			}

			Ok(None)
		})
	}
);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CheckPackageCompatabilityKeys {
	pub item: ConfiguredItem,
	pub package: ArcPkgReq,
}

#[derive(Clone)]
pub enum PackageCompatabilityError {
	WrongMinecraftVersion,
	WrongLoader,
}
