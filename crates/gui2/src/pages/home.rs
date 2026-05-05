use std::{hash::Hash, time::Duration};

use crate::{components::instance::InstanceListItem, prelude::*};
use freya::query::Captured;
use itertools::Itertools;
use nitrolaunch::{
	config_crate::ConfigKind, core::util::versions::MinecraftVersion, instance::parse_loader_config,
};

use crate::components::instance::InstanceItemInfo;

#[derive(PartialEq)]
pub struct HomePage;

impl Component for HomePage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let app_state = use_radio(AppChannel::Default);
		let items_query = use_query(
			Query::new(
				(),
				FetchItems {
					app_state: Captured(app_state.read().cloned()),
				},
			)
			.stale_time(Duration::from_secs(3)),
		);

		let mut tab = use_state(|| Tab::Instances);
		let selected = use_state::<Option<SelectedLocation>>(|| None);

		let items_gap = 20.0;
		let items_side_padding = 32.0;
		let items = items_query.read();
		let items = match &*items.state() {
			QueryStateData::Pending
			| QueryStateData::Loading { res: _ }
			| QueryStateData::Settled { res: Err(..), .. } => InstancesAndTemplates {
				instances: Vec::new(),
				templates: Vec::new(),
			},
			QueryStateData::Settled { res: Ok(res), .. } => res.clone(),
		};

		let items = match &*tab.read() {
			Tab::Instances => &items.instances,
			Tab::Templates => &items.templates,
		};

		let items = items
			.into_iter()
			.map(|x| InstanceListItem::new(x.clone(), selected.clone()));

		let items_elem = grid(3, items).gap(items_gap);

		let items_elem = rect().child(items_elem).width(Size::fill());

		let tab_color = if &*tab.read() == &Tab::Instances {
			theme.primary.into()
		} else {
			theme.disabled.into()
		};

		let instances_tab = rect()
			.cont()
			.center()
			.corner_radius(theme.round)
			.width(Size::px(128.0))
			.height(Size::fill())
			.color(tab_color)
			.border(Some(Border {
				fill: tab_color,
				width: theme.border.into(),
				alignment: BorderAlignment::Center,
			}))
			.background(theme.item)
			.on_press(move |_| tab.set(Tab::Instances))
			.clickable()
			.child(icon("box", 16.0))
			.child("Instances");

		let tab_color = if &*tab.read() == &Tab::Templates {
			theme.primary.into()
		} else {
			theme.disabled.into()
		};

		let templates_tab = rect()
			.cont()
			.center()
			.corner_radius(theme.round)
			.width(Size::px(128.0))
			.height(Size::fill())
			.color(tab_color)
			.border(Some(Border {
				fill: tab_color,
				width: theme.border.into(),
				alignment: BorderAlignment::Center,
			}))
			.background(theme.item)
			.on_press(move |_| tab.set(Tab::Templates))
			.clickable()
			.child(icon("diagram", 16.0))
			.child("Templates");

		let bar_left = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.cont()
			.spacing(12.0)
			.cross_align(Alignment::Center)
			.child(instances_tab)
			.child(templates_tab);

		let bar_center = rect().width(Size::flex(1.0));
		let bar_right = rect().width(Size::flex(1.0));

		let bar_elem = rect()
			.width(Size::fill())
			.height(Size::px(32.0))
			.cont()
			.padding((3.0, items_gap))
			.child(bar_left)
			.child(bar_center)
			.child(bar_right);

		let view = rect().flex().child(bar_elem).child(items_elem);

		let view = ScrollView::new()
			.child(view)
			.width(Size::fill())
			.height(Size::fill());

		rect().fill().child(view).padding((0.0, items_side_padding))
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct FetchItems {
	app_state: Captured<AppState>,
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

#[derive(PartialEq)]
enum Tab {
	Instances,
	Templates,
}

#[derive(Clone)]
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
