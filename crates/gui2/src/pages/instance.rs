use nitrolaunch::config_crate::{ConfigKind, template::TemplateConfig};

use crate::{
	components::{input::tabs::SideTabs, instance::console::InstanceConsole},
	ops::instance::{FetchInstanceConfig, FetchParentConfigs, SaveConfig},
	pages::config::{ConfigState, addons::AddonsConfig},
	prelude::*,
	util::{PtrEq, assets::get_instance_icon},
};

#[derive(PartialEq)]
pub struct InstancePage {
	pub id: String,
}

impl Component for InstancePage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let config_query = use_query(FetchInstanceConfig::new(
			self.id.clone(),
			back_state.clone(),
		));
		let save_config = use_mutation(Mutation::new(SaveConfig::new(back_state.clone()).toast(
			&back_state,
			Some("Saved"),
			"Failed to save config",
		)));

		let tab = use_state(|| Tab::Info);
		let is_dirty = use_state(|| false);

		let id = self.id.clone();
		let config_state = ConfigState::new(ConfigKind::Instance, false, is_dirty);
		let mut config_state2 = config_state.clone();
		use_side_effect(move || {
			let config = config_query
				.read()
				.state()
				.ok()
				.cloned()
				.flatten()
				.unwrap_or_default();

			let template_config = TemplateConfig {
				instance: config.no_templates,
				..Default::default()
			};

			config_state2.update(Some(id.clone()), template_config);
		});

		let parent_configs = use_query(FetchParentConfigs::new(
			config_state.from.read().cloned(),
			back_state.clone(),
		));
		let parent_configs = parent_configs
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();

		let ico = if config_state.icon.read().is_some() {
			ImageViewer::new(get_instance_icon(config_state.icon.read().as_deref()))
				.width(Size::px(64.0))
				.height(Size::px(64.0))
				.corner_radius(theme.round2)
				.into_element()
		} else {
			icon("box", 48.0).into_element()
		};
		let ico = rect()
			.width(Size::px(96.0))
			.height(Size::px(96.0))
			.center()
			.child(ico);
		let name = config_state
			.name
			.read()
			.cloned()
			.unwrap_or_else(|| self.id.clone());
		let head = rect()
			.width(Size::fill())
			.height(Size::px(96.0))
			.cont()
			.border(border_bottom(theme.border, theme.panel_border))
			.child(ico)
			.child(
				segment(name, 1.0)
					.height(Size::fill())
					.font_size(theme.font2)
					.font_weight(FontWeight::BOLD)
					.main_align(Alignment::Center),
			);

		let tabs = SideTabs::new(tab)
			.child(SelectOption::new(Tab::Info, "Info", Some("info")))
			.child(SelectOption::new(
				Tab::Content,
				"Content",
				Some("honeycomb"),
			))
			.child(SelectOption::new(Tab::Console, "Console", Some("text")));
		let tabs = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.border(border_right(theme.border, theme.panel_border))
			.child(tabs);

		let contents = match &*tab.read() {
			Tab::Info => rect().into_element(),
			Tab::Content => AddonsConfig {
				config_state: config_state.clone(),
				parent_configs: PtrEq(parent_configs.clone()),
			}
			.into_element(),
			Tab::Console => InstanceConsole {
				id: self.id.clone(),
			}
			.into_element(),
		};
		let contents = rect()
			.width(Size::flex(5.0))
			.height(Size::fill())
			.child(contents);

		let body = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.flex()
			.horizontal()
			.child(tabs)
			.child(contents);

		rect().expanded().flex().child(head).child(body)
	}
}

#[derive(PartialEq, Clone)]
enum Tab {
	Info,
	Content,
	Console,
}
