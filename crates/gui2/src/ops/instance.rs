use std::{sync::Arc, time::Duration};

use anyhow::Context;
use itertools::Itertools;
use nitrolaunch::{
	config::modifications::{ConfigModification, apply_modifications_and_write},
	config_crate::{ConfigKind, instance::InstanceConfig, template::TemplateConfig},
	core::util::versions::MinecraftVersion,
	instance::parse_loader_config,
	shared::{
		Side,
		id::{InstanceID, TemplateID},
		loaders::Loader,
		output::NoOp,
	},
};

use crate::{pages::config::ConfiguredItem, prelude::*};

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
				editable: instance.original_config().clone(),
			}))
		})
	}
}

pub struct InstanceConfigs {
	pub main: InstanceConfig,
	pub editable: InstanceConfig,
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
}
