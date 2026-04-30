use crate::prelude::*;
use gpui_component::tab::TabBar;
use itertools::Itertools;
use nitrolaunch::{
	config_crate::ConfigKind, core::util::versions::MinecraftVersion, instance::parse_loader_config,
};

use crate::components::instance::{InstanceItemInfo, InstanceListItem};

pub struct HomePage {
	app_state: AppState,
	items: Resource<InstancesAndTemplates>,
	visible_trigger: Trigger,
	tab: Tab,
	selected_item: Option<SelectedLocation>,
}

impl HomePage {
	pub fn new(app_state: AppState, _: &Window, _: &mut Context<Self>) -> Self {
		Self {
			app_state,
			items: Resource::new(),
			visible_trigger: Trigger::new(),
			tab: Tab::Instances,
			selected_item: None,
		}
	}

	fn fetch_items(&self, cx: &mut Context<Self>) {
		let app_state = self.app_state.clone();

		self.items.fetch(cx, async move |_| {
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
		});
	}

	pub fn visible(&mut self) {
		self.visible_trigger.trigger();
	}

	pub fn select(&mut self, selected: SelectedLocation, cx: &mut Context<Self>) {
		self.selected_item = Some(selected);
		cx.notify();
	}
}

impl Render for HomePage {
	fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		if self.visible_trigger.check() {
			self.fetch_items(cx);
		}

		let items = match self.items.state().as_deref() {
			None | Some(ResourceState::Loading | ResourceState::Err(..)) => {
				vec![div().into_any_element()]
			}
			Some(ResourceState::Loaded(items)) => {
				let items = match &self.tab {
					Tab::Instances => &items.instances,
					Tab::Templates => &items.templates,
				};

				items
					.iter()
					.map(|x| {
						let entity = cx.entity();
						let selected = self
							.selected_item
							.as_ref()
							.is_some_and(|y| y.is_selected(x));

						cx.new(|cx| InstanceListItem::new(x.clone(), selected, entity, window, cx))
							.into_any_element()
					})
					.collect()
			}
		};

		let entity = cx.entity();
		let tabs = TabBar::new("home-tabs")
			.selected_index(self.tab.to_index())
			.on_click(move |idx, _, cx| {
				entity.update(cx, |this, cx| {
					this.tab = Tab::from_index(*idx);
					cx.notify();
				});
			})
			.child(gpui_component::tab::Tab::new().label("Instances"))
			.child(gpui_component::tab::Tab::new().label("Templates"));

		rsx! {
			<v_flex id="home-container" size_full overflow_y_scrollbar gap_3>
				<div>{tabs}</div>
				<div grid grid_cols=5 gap_5 p_3 px_8>{...items}</div>
			</v_flex>
		}
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
	fn is_selected(&self, info: &InstanceItemInfo) -> bool {
		info.id == self.id && info.ty == self.ty
	}
}
