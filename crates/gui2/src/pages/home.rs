use std::hash::Hash;

use crate::{components::instance::InstanceListItem, prelude::*};
use itertools::Itertools;
use nitrolaunch::{
	config_crate::ConfigKind, core::util::versions::MinecraftVersion, instance::parse_loader_config,
};

use crate::components::instance::InstanceItemInfo;

#[derive(PartialEq)]
pub struct HomePage;

impl Component for HomePage {
	fn render(&self) -> impl IntoElement {
		let app_state = use_radio(AppChannel::Default);
		let items_query = use_query(Query::new(
			(),
			FetchItems {
				app_state: app_state.read().cloned(),
			},
		));

		let tab = use_state(|| Tab::Instances);
		let selected = use_state::<Option<SelectedLocation>>(|| None);

		let items = items_query.read();
		let items_elem = match &*items.state() {
			QueryStateData::Pending
			| QueryStateData::Loading { res: _ }
			| QueryStateData::Settled { res: Err(..), .. } => rect().into_element(),
			QueryStateData::Settled { res: Ok(res), .. } => {
				let items = match &*tab.read() {
					Tab::Instances => &res.instances,
					Tab::Templates => &res.templates,
				};

				let items = items
					.into_iter()
					.map(|x| InstanceListItem::new(x.clone(), selected.clone()));

				grid(4, items).gap(15.0).into_element()
			}
		};

		let items_elem = ScrollView::new().child(items_elem);

		rect().fill().child(items_elem)
	}
}

#[derive(Clone)]
struct FetchItems {
	app_state: AppState,
}

impl PartialEq for FetchItems {
	fn eq(&self, _: &Self) -> bool {
		true
	}
}

impl Eq for FetchItems {}

impl Hash for FetchItems {
	fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl QueryCapability for FetchItems {
	type Ok = InstancesAndTemplates;
	type Err = anyhow::Error;
	type Keys = ();

	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let app_state = self.app_state.clone();

		query_spawn(async move {
			let config = app_state.config().await?;

			let instances = config
				.instances
				.values()
				.sorted_by_cached_key(|x| x.id())
				.map(|x| InstanceItemInfo {
					id: x.id().to_string(),
					ty: ConfigKind::Instance,
					name: x.config().name.clone(),
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

enum Tab {
	Instances,
	Templates,
}

impl Tab {
	fn to_index(&self) -> usize {
		match self {
			Self::Instances => 0,
			Self::Templates => 1,
		}
	}

	fn from_index(idx: usize) -> Self {
		match idx {
			0 => Self::Instances,
			1 => Self::Templates,
			_ => unreachable!(),
		}
	}
}

struct InstancesAndTemplates {
	instances: Vec<InstanceItemInfo>,
	templates: Vec<InstanceItemInfo>,
}

#[derive(Clone)]
pub struct SelectedLocation {
	pub id: String,
	pub ty: ConfigKind,
}

impl SelectedLocation {
	pub fn from_item(info: &InstanceItemInfo) -> Self {
		Self {
			id: info.id.clone(),
			ty: info.ty.clone(),
		}
	}

	pub fn is_selected(&self, info: &InstanceItemInfo) -> bool {
		info.id == self.id && info.ty == self.ty
	}
}
