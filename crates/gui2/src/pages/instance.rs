use std::rc::Rc;

use nitrolaunch::config_crate::{ConfigKind, template::TemplateConfig};

use crate::{
	components::{
		input::{select::Selected, tabs::SideTabs},
		instance::console::InstanceConsole,
	},
	ops::{
		instance::{FetchInstanceConfig, FetchParentConfigs, SaveConfig},
		launch::FetchInstanceRunState,
	},
	pages::config::{ConfigState, ConfiguredItem, addons::AddonsConfig},
	prelude::*,
	state::ModalType,
	util::{PtrEq, assets::get_instance_icon},
};

#[derive(PartialEq)]
pub struct InstancePage {
	pub id: String,
}

impl Component for InstancePage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
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
		let run_state = use_query(Query::new(
			self.id.clone(),
			FetchInstanceRunState::new(back_state.clone()),
		));

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

		let id = self.id.clone();
		let front_state2 = front_state.clone();
		let settings_button =
			rect()
				.tip(&front_state, "Configure")
				.child(icon_button("gear", &theme).on_press(move |_| {
					front_state2
						.write()
						.set_modal(Some(ModalType::Configuration(ConfiguredItem {
							ty: ConfigKind::Instance,
							id: Some(id.clone()),
							is_new: false,
						})));
				}));

		let id = self.id.clone();
		let front_state2 = front_state.clone();
		let more_dropdown = Dropdown::new(
			Selected::Single(MoreOption::More),
			Rc::new(move |selected| match selected.single() {
				MoreOption::More => {}
				MoreOption::Delete => {
					front_state2
						.write()
						.set_modal(Some(ModalType::DeleteInstance(id.clone())));
				}
			}),
		)
		.options_width(160.0)
		.align_options_right()
		.custom_header(SelectOption::new(MoreOption::More, "More", None))
		.child(SelectOption::new(
			MoreOption::Delete,
			"Delete",
			Some("trash"),
		));
		let more_dropdown = rect().width(Size::px(84.0)).child(more_dropdown);

		let controls = rect()
			.height(Size::fill())
			.cont()
			.main_align(Alignment::End)
			.cross_align(Alignment::Center)
			.padding(Gaps::new(0.0, theme.gap3, 0.0, 0.0))
			.child(settings_button)
			.child(more_dropdown);

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
			)
			.child(controls);

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

#[derive(PartialEq, Clone)]
enum MoreOption {
	More,
	Delete,
}
