use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail};
use freya::query::QueriesStorage;
use itertools::Itertools;
use nitrolaunch::{
	config::modifications::{ConfigModification, apply_modifications_and_write},
	config_crate::{ConfigKind, instance::InstanceConfig, template::TemplateConfig},
	core::util::versions::MinecraftVersion,
	instance::{
		parse_loader_config,
		update::{InstanceUpdateContext, UpdateFacets, manager::UpdateSettings},
	},
	io::lock::Lockfile,
	shared::{
		Side, UpdateDepth,
		id::{InstanceID, TemplateID},
		loaders::Loader,
		output::NoOp,
	},
};

use crate::{
	ops::{MakeSend, task::Task},
	pages::config::ConfiguredItem,
	prelude::*,
	secrets::get_ms_client_id,
	simple_mutation, simple_query,
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchItems {
	back_state: Captured<BackState>,
}

impl FetchItems {
	pub fn new(back_state: BackState) -> Query<Self> {
		Query::new(
			(),
			Self {
				back_state: Captured(back_state),
			},
		)
		.stale_time(Duration::from_secs(30))
	}
}

impl QueryCapability for FetchItems {
	type Ok = InstancesAndTemplates;
	type Err = anyhow::Error;
	type Keys = ();

	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let config = back_state.config().await?;

			let instances = config
				.instances
				.values()
				.sorted_by_cached_key(|x| x.id())
				.map(|x| InstanceItemInfo {
					id: x.id().to_string(),
					ty: ConfigKind::Instance,
					name: x.config().name.clone(),
					icon: x.config().icon.clone(),
					side: Some(x.side()),
					version: Some(x.version().clone()),
					loader: Some(x.loader().clone()),
				});

			let templates = config
				.consolidated_templates
				.iter()
				.sorted_by_cached_key(|x| x.0.clone())
				.map(|(id, x)| InstanceItemInfo {
					id: id.to_string(),
					ty: ConfigKind::Template,
					name: x.instance.name.clone(),
					icon: x.instance.icon.clone(),
					side: x.instance.side,
					version: x
						.instance
						.version
						.as_ref()
						.map(|x| MinecraftVersion::from_deser(&x)),
					loader: x.instance.loader.as_ref().map(|x| parse_loader_config(x).0),
				});

			let base_template = InstanceItemInfo {
				id: "base".into(),
				ty: ConfigKind::BaseTemplate,
				name: Some("Base Template".into()),
				icon: None,
				side: None,
				version: None,
				loader: None,
			};

			Ok(InstancesAndTemplates {
				instances: instances.collect(),
				templates: std::iter::once(base_template).chain(templates).collect(),
			})
		})
	}
}

/// Simple info about an instance or template
#[derive(Clone, PartialEq)]
pub struct InstanceItemInfo {
	pub id: String,
	pub ty: ConfigKind,
	pub name: Option<String>,
	pub icon: Option<String>,
	pub side: Option<Side>,
	pub version: Option<MinecraftVersion>,
	pub loader: Option<Loader>,
}

impl InstanceItemInfo {
	pub fn get_config_item(&self) -> ConfiguredItem {
		let id = if self.ty == ConfigKind::BaseTemplate {
			None
		} else {
			Some(self.id.clone())
		};

		ConfiguredItem {
			id,
			ty: self.ty,
			is_new: false,
		}
	}

	pub fn name(&self) -> &str {
		self.name.as_deref().unwrap_or(self.id.as_str())
	}
}

#[derive(Clone, Default)]
pub struct InstancesAndTemplates {
	pub instances: Vec<InstanceItemInfo>,
	pub templates: Vec<InstanceItemInfo>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchInstanceConfig {
	back_state: Captured<BackState>,
}

impl FetchInstanceConfig {
	pub fn new(id: String, back_state: BackState) -> Query<Self> {
		Query::new(
			id,
			Self {
				back_state: Captured(back_state),
			},
		)
	}
}

impl QueryCapability for FetchInstanceConfig {
	type Ok = Option<InstanceConfigs>;
	type Err = anyhow::Error;
	type Keys = String;

	fn run(&self, id: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = id.clone();

		query_spawn(async move {
			let config = back_state.config().await?;

			let Some(instance) = config.instances.get(&InstanceID::from(id)) else {
				return Ok(None);
			};

			Ok(Some(InstanceConfigs {
				main: instance.config().clone(),
				no_templates: instance.original_config().clone(),
			}))
		})
	}
}

#[derive(Clone, Default)]
pub struct InstanceConfigs {
	pub main: InstanceConfig,
	pub no_templates: InstanceConfig,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchInstanceOrTemplateConfig {
	back_state: Captured<BackState>,
}

impl FetchInstanceOrTemplateConfig {
	pub fn new(item: ConfiguredItem, back_state: BackState) -> Query<Self> {
		Query::new(
			item,
			Self {
				back_state: Captured(back_state),
			},
		)
	}
}

impl QueryCapability for FetchInstanceOrTemplateConfig {
	type Ok = Option<InstanceOrTemplateConfigs>;
	type Err = anyhow::Error;
	type Keys = ConfiguredItem;

	fn run(&self, item: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let item = item.clone();

		query_spawn(async move {
			let config = back_state.config().await?;

			match item.ty {
				ConfigKind::Instance => {
					let Some(instance) = config
						.instances
						.get(&InstanceID::from(item.id.context("ID missing")?))
					else {
						return Ok(None);
					};

					Ok(Some(InstanceOrTemplateConfigs {
						main: TemplateConfig::from_instance(instance.config().clone()),
						no_templates: TemplateConfig::from_instance(
							instance.original_config().clone(),
						),
					}))
				}
				ConfigKind::Template => {
					let id = TemplateID::from(item.id.context("ID missing")?);
					let Some(template) = config.templates.get(&id) else {
						return Ok(None);
					};
					let Some(consolidated_template) = config.consolidated_templates.get(&id) else {
						return Ok(None);
					};

					Ok(Some(InstanceOrTemplateConfigs {
						main: consolidated_template.clone(),
						no_templates: template.clone(),
					}))
				}
				ConfigKind::BaseTemplate => Ok(Some(InstanceOrTemplateConfigs {
					main: config.base_template.clone(),
					no_templates: config.base_template.clone(),
				})),
			}
		})
	}
}

#[derive(Default, Clone)]
pub struct InstanceOrTemplateConfigs {
	pub main: TemplateConfig,
	pub no_templates: TemplateConfig,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchParentConfigs {
	back_state: Captured<BackState>,
}

impl FetchParentConfigs {
	pub fn new(from: Vec<String>, back_state: BackState) -> Query<Self> {
		Query::new(
			from,
			Self {
				back_state: Captured(back_state),
			},
		)
	}
}

impl QueryCapability for FetchParentConfigs {
	type Ok = Arc<[TemplateConfig]>;
	type Err = anyhow::Error;
	type Keys = Vec<String>;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let from = keys.clone();

		query_spawn(async move {
			let config = back_state.config().await?;

			let out = from
				.iter()
				.filter_map(|x| {
					config
						.consolidated_templates
						.get(&TemplateID::from(x.clone()))
						.cloned()
				})
				.collect();

			Ok(out)
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SaveConfig {
	back_state: Captured<BackState>,
}

#[derive(Clone, PartialEq, Hash)]
pub struct SaveConfigParams {
	pub item: ConfiguredItem,
	pub config: NotEq<TemplateConfig>,
}

impl SaveConfig {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

impl MutationCapability for SaveConfig {
	type Ok = ();
	type Err = anyhow::Error;
	type Keys = SaveConfigParams;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let keys = keys.clone();
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let mut raw_config = back_state.raw_config().await?;
			let modification = match keys.item.ty {
				ConfigKind::Instance if keys.item.is_new => ConfigModification::AddInstance(
					keys.item.id.unwrap().into(),
					keys.config.0.instance,
				),
				ConfigKind::Instance => ConfigModification::UpdateInstance(
					keys.item.id.unwrap().into(),
					keys.config.0.instance,
				),
				ConfigKind::Template if keys.item.is_new => {
					ConfigModification::AddTemplate(keys.item.id.unwrap().into(), keys.config.0)
				}
				ConfigKind::Template => {
					ConfigModification::UpdateTemplate(keys.item.id.unwrap().into(), keys.config.0)
				}
				ConfigKind::BaseTemplate => {
					raw_config.base_template = Some(keys.config.0);
					apply_modifications_and_write(
						&mut raw_config,
						Vec::new(),
						&back_state.paths,
						&back_state.plugins,
						&mut NoOp,
					)
					.await?;
					return Ok(());
				}
			};

			apply_modifications_and_write(
				&mut raw_config,
				vec![modification],
				&back_state.paths,
				&back_state.plugins,
				&mut NoOp,
			)
			.await?;

			Ok(())
		})
	}

	fn on_settled(
		&self,
		keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		async {
			QueriesStorage::<FetchItems>::invalidate_all().await;
			if let Some(id) = &keys.item.id {
				QueriesStorage::<FetchInstanceConfig>::invalidate_matching(id.clone()).await;
			}
			QueriesStorage::<FetchInstanceOrTemplateConfig>::invalidate_matching(keys.item.clone())
				.await;
		}
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchInstanceOutput {
	back_state: Captured<BackState>,
}

impl FetchInstanceOutput {
	pub fn new(id: String, log_file: Option<String>, back_state: BackState) -> Query<Self> {
		Query::new(
			(id, log_file),
			Self {
				back_state: Captured(back_state),
			},
		)
	}
}

impl QueryCapability for FetchInstanceOutput {
	type Ok = Option<Arc<str>>;
	type Err = anyhow::Error;
	type Keys = (String, Option<String>);

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let (id, log_file) = keys.clone();

		query_spawn(async move {
			if let Some(log_file) = log_file {
				let config = back_state.config().await?;
				let mut output = back_state.output();
				let instance = config
					.instances
					.get(&InstanceID::from(id.clone()))
					.context("Instance not found")?;

				return Ok(Some(
					instance
						.get_log(
							&log_file,
							&back_state.plugins,
							&back_state.paths,
							&mut output,
						)
						.await?
						.into(),
				));
			}

			let path = {
				let Some(entry) = back_state.running_instances.get_entry(&id, None).await else {
					return Ok(None);
				};

				let Some(path) = &entry.stdout_file else {
					return Ok(None);
				};

				back_state.paths.internal.join("stdio").join(path)
			};

			let contents = tokio::fs::read_to_string(path)
				.await
				.context("Failed to read output file")?;

			Ok(Some(contents.into()))
		})
	}
}

simple_query!(
	name = FetchInstanceLogs,
	ok = Arc<[String]>,
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move {
			let config = back_state.config().await?;
			let mut o = back_state.output();
			let instance = config
				.instances
				.get(&InstanceID::from(id.clone()))
				.context("Instance not found")?;

			let logs = instance.get_logs(&back_state.plugins, &back_state.paths, &mut o).await?;

			Ok(logs.into_iter().collect())
		})
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = UpdateInstance,
	ok = (),
	err = anyhow::Error,
	keys = UpdateInstanceKeys,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let task_id = if keys.content_only {
			Task::UpdateInstanceContent(keys.id.clone())
		} else {
			Task::UpdateInstance(keys.id.clone())
		};

		let facets = if keys.content_only {
			UpdateFacets::content()
		} else {
			UpdateFacets::all()
		};

		let depth = if keys.force {
			UpdateDepth::Force
		} else {
			UpdateDepth::Full
		};

		update_instance_impl(
			self.back_state.0.clone(),
			keys.id.clone(),
			depth,
			facets,
			task_id,
		)
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		async move {
			QueriesStorage::<FetchInstanceConfig>::invalidate_matching(_keys.id.clone()).await;
			QueriesStorage::<FetchItems>::invalidate_all().await;
		}
	}
);

pub async fn update_instance_impl(
	back_state: BackState,
	instance_id: String,
	depth: UpdateDepth,
	facets: UpdateFacets,
	task_id: Task,
) -> anyhow::Result<()> {
	let mut o = back_state.output();
	o.set_task(task_id.clone());

	let back_state2 = back_state.clone();
	let task = {
		let mut config = back_state2.config().await?;
		let mut lock = Lockfile::open(&back_state2.paths).context("Failed to open lockfile")?;

		let core = config
			.get_core(
				Some(&get_ms_client_id()),
				&UpdateSettings {
					depth: UpdateDepth::Full,
					offline_auth: false,
				},
				&back_state2.client,
				&back_state2.plugins,
				&back_state2.paths,
				&mut o,
			)
			.await?;

		let instance_id = instance_id.clone();
		let paths = back_state2.paths.clone();
		async move {
			let instance_id2 = instance_id.clone();
			let Some(instance) = config.instances.get_mut(&InstanceID::from(instance_id)) else {
				bail!("Instance does not exist");
			};

			let mut ctx = InstanceUpdateContext {
				packages: &mut config.packages,
				accounts: &mut config.accounts,
				plugins: &config.plugins,
				prefs: &config.prefs,
				paths: &paths,
				lock: &mut lock,
				client: &back_state2.client,
				output: &mut o,
				core: &core,
			};

			let updates_packages = facets.packages;

			instance
				.update(depth, facets, &mut ctx)
				.await
				.context("Failed to update instance")?;

			if updates_packages {
				let mut data = back_state2.data();
				data.last_resolution_errors.remove(&instance_id2);
				let _ = data.write(&paths);
			}

			Ok(())
		}
	};

	let task = tokio::spawn(unsafe { MakeSend::new(task) });
	back_state.register_task(task_id, task);

	Ok(())
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UpdateInstanceKeys {
	pub id: String,
	pub force: bool,
	pub content_only: bool,
}

#[rustfmt::skip]
simple_mutation!(
	name = DeleteInstance,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move {
			let config = back_state.config().await?;
			let mut o = back_state.output();
			o.set_task(Task::DeleteInstance);
			let instance = config
				.instances
				.get(&InstanceID::from(id.clone()))
				.context("Instance not found")?;
			instance
				.delete(&back_state.paths, &back_state.plugins, &mut o)
				.await
				.context("Failed to delete instance")
		})
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		QueriesStorage::<FetchItems>::invalidate_all()
	}
);
