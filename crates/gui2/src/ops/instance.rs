use std::time::Duration;

use anyhow::Context;
use itertools::Itertools;
use nitrolaunch::{
	config_crate::{ConfigKind, instance::InstanceConfig, template::TemplateConfig},
	core::util::versions::MinecraftVersion,
	instance::parse_loader_config,
	shared::{
		Side,
		id::{InstanceID, TemplateID},
		loaders::Loader,
	},
};

use crate::{pages::instance::config::ConfiguredItem, prelude::*};

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
						.get(&InstanceID::from(item.id.context("ID mising")?))
					else {
						return Ok(None);
					};

					Ok(Some(InstanceOrTemplateConfigs {
						main: TemplateConfig::from_instance(instance.config().clone()),
						editable: TemplateConfig::from_instance(instance.original_config().clone()),
					}))
				}
				ConfigKind::Template => {
					let id = TemplateID::from(item.id.context("ID mising")?);
					let Some(template) = config.templates.get(&id) else {
						return Ok(None);
					};
					let Some(consolidated_template) = config.consolidated_templates.get(&id) else {
						return Ok(None);
					};

					Ok(Some(InstanceOrTemplateConfigs {
						main: consolidated_template.clone(),
						editable: template.clone(),
					}))
				}
				ConfigKind::BaseTemplate => Ok(Some(InstanceOrTemplateConfigs {
					main: config.base_template.clone(),
					editable: config.base_template.clone(),
				})),
			}
		})
	}
}

#[derive(Default, Clone)]
pub struct InstanceOrTemplateConfigs {
	pub main: TemplateConfig,
	pub editable: TemplateConfig,
}
