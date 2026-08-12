use std::rc::Rc;

use nitrolaunch::{
	config_crate::{ConfigKind, instance::InstanceConfig, template::TemplateConfig},
	plugin_crate::hook::hooks::DropdownButtonLocation,
};

use crate::{
	components::{
		input::{
			select::{Selected, run_dropdown_button},
			tabs::SideTabs,
		},
		instance::{console::InstanceConsole, transfer::InstanceTransferMode},
	},
	ops::{
		instance::{
			FetchInstanceConfig, FetchParentConfigs, SaveConfig, UpdateInstance,
			UpdateInstanceKeys, UpdateInstanceMode,
		},
		launch::{
			FetchInstanceRunState, InstanceRunState, KillInstance, LaunchInstance,
			LaunchInstanceParams,
		},
		misc::{ShowDirectory, ShowDirectoryOption},
		plugin_results::{FetchDropdownButtons, OpenCustomPopup, RunCustomAction},
	},
	pages::config::{ConfigState, ConfiguredItem, content::ContentConfig},
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
		let update = use_mutation(Mutation::new(UpdateInstance::new(back_state.clone())));
		let launch = use_mutation(LaunchInstance::new(back_state.clone()));
		let kill = use_mutation(KillInstance::new(back_state.clone()));
		let run_state = use_query(Query::new(
			self.id.clone(),
			FetchInstanceRunState::new(back_state.clone()),
		));
		let run_state = run_state.read().state().ok().cloned().unwrap_or_default();
		let show_directory = use_mutation(Mutation::new(ShowDirectory::new(back_state.clone())));
		let more_buttons = use_query(Query::new(
			DropdownButtonLocation::InstanceMoreOptions,
			FetchDropdownButtons::new(back_state.clone()),
		));
		let custom_action_mutation = use_mutation(Mutation::new(
			RunCustomAction::new(back_state.clone()).toast(
				&back_state,
				None,
				"Failed to run action",
			),
		));
		let open_popup_mutation = use_mutation(Mutation::new(OpenCustomPopup::new(
			back_state.clone(),
			front_state.clone(),
		)));

		let tab = use_state(|| Tab::Console);
		let config = use_state(InstanceConfig::default);

		let id = self.id.clone();
		let config_state = ConfigState::new(ConfigKind::Instance, false);
		let mut config_state2 = config_state.clone();
		let mut config2 = config;
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
			config2.set(config.main);
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
		config_state
			.parent_configs
			.clone()
			.set_if_modified(PtrEq(parent_configs.clone()));

		let is_editable = config.read().is_editable || config_state.plugin.peek().is_none();
		let is_deletable = config.read().is_deletable || config_state.plugin.peek().is_none();

		let ico = if config.read().icon.is_some() {
			ImageViewer::new(get_instance_icon(config.read().icon.as_deref()))
				.width(Size::px(48.0))
				.height(Size::px(48.0))
				.corner_radius(theme.round2)
				.into_element()
		} else {
			icon("box_dot", 36.0).into_element()
		};
		let ico = rect()
			.width(Size::px(80.0))
			.height(Size::px(80.0))
			.center()
			.child(ico);
		let name = config
			.read()
			.name
			.clone()
			.unwrap_or_else(|| self.id.clone());

		let selected = match run_state {
			InstanceRunState::Stopped => LaunchOption::Launch,
			InstanceRunState::Running => LaunchOption::Kill,
		};
		let id = self.id.clone();
		let launch_dropdown = Dropdown::new(
			Selected::Single(selected),
			Rc::new(move |selected| match selected.single() {
				LaunchOption::Launch => {
					launch.mutate(LaunchInstanceParams {
						id: id.clone(),
						offline: false,
					});
				}
				LaunchOption::LaunchOffline => {
					launch.mutate(LaunchInstanceParams {
						id: id.clone(),
						offline: true,
					});
				}
				LaunchOption::Kill => {
					kill.mutate((id.clone(), None));
				}
			}),
		)
		.options_width(160.0)
		.header_width(Size::auto())
		.panel_colorway()
		.child(SelectOption::new(
			LaunchOption::Launch,
			"Launch",
			Some("play"),
		))
		.child(SelectOption::new(
			LaunchOption::LaunchOffline,
			"Launch Offline",
			Some("play"),
		))
		.maybe_child(run_state == InstanceRunState::Running, || {
			SelectOption::new(LaunchOption::Kill, "Kill", Some("stop"))
		});

		let id = self.id.clone();
		let update_button = icon_text_button("cycle", "Update", &theme)
			.border_fill(theme.panel_border)
			.on_press(move |_| {
				update.mutate(UpdateInstanceKeys {
					id: id.clone(),
					mode: UpdateInstanceMode::Full,
					force: false,
				});
			});

		let id = self.id.clone();
		let front_state2 = front_state.clone();
		let settings_button = icon_text_button("gear", "Configure", &theme)
			.border_fill(theme.panel_border)
			.on_press(move |_| {
				front_state2
					.write()
					.set_modal(Some(ModalType::Configuration(ConfiguredItem {
						ty: ConfigKind::Instance,
						id: Some(id.clone()),
						is_new: false,
					})));
			});

		let id = self.id.clone();
		let front_state2 = front_state.clone();
		let more_buttons = more_buttons.read();
		let more_buttons = more_buttons.state();
		let more_buttons = more_buttons.ok().cloned().unwrap_or_default();
		let more_buttons2 = more_buttons.clone();
		let more_dropdown = Dropdown::new(
			Selected::Single(MoreOption::More),
			Rc::new(move |selected| match selected.single() {
				MoreOption::More => {}
				MoreOption::Export => {
					front_state2.write().set_modal(Some(ModalType::Transfer(
						InstanceTransferMode::Export,
						Some(id.clone()),
					)));
				}
				MoreOption::OpenFolder => {
					show_directory.mutate(ShowDirectoryOption::Instance(id.clone()));
				}
				MoreOption::Delete => {
					front_state2
						.write()
						.set_modal(Some(ModalType::DeleteInstance(id.clone())));
				}
				MoreOption::Custom(idx) => {
					if let Some(button) = more_buttons2.get(idx) {
						run_dropdown_button(
							button,
							Some(id.clone()),
							&custom_action_mutation,
							&open_popup_mutation,
						);
					}
				}
			}),
		)
		.options_width(160.0)
		.align_options_right()
		.custom_header(SelectOption::new(MoreOption::More, "More", Some("ellipsis")))
		.header_width(Size::auto())
		.hide_arrow()
		.panel_colorway()
		.child(SelectOption::new(
			MoreOption::Export,
			"Export",
			Some("popout"),
		))
		.child(SelectOption::new(
			MoreOption::OpenFolder,
			"Open Folder",
			Some("folder"),
		))
		.maybe_child(is_deletable, || {
			SelectOption::new(MoreOption::Delete, "Delete", Some("trash"))
		})
		.custom_buttons(more_buttons, MoreOption::Custom);

		let controls = rect()
			.height(Size::fill())
			.cont()
			.main_align(Alignment::End)
			.cross_align(Alignment::Center)
			.padding(Gaps::new(0.0, theme.gap3, 0.0, 0.0))
			.child(launch_dropdown)
			.child(update_button)
			.maybe(is_editable, |this| this.child(settings_button))
			.child(more_dropdown);

		let head = rect()
			.width(Size::fill())
			.height(Size::px(80.0))
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
			.child(SelectOption::new(Tab::Console, "Console", Some("text")))
			.child(SelectOption::new(
				Tab::Content,
				"Content",
				Some("honeycomb"),
			));
		let tabs = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.border(border_right(theme.border, theme.panel_border))
			.child(tabs);

		let config_state2 = config_state.clone();
		let save_fn = config_state.save_fn(front_state.clone(), save_config);
		let contents = match &*tab.read() {
			// Tab::Info => rect().into_element(),
			Tab::Content => ContentConfig {
				config_state: config_state.clone(),
				parent_configs: PtrEq(parent_configs.clone()),
				on_edit: Some(
					(move |_| {
						if config_state2.has_changed() {
							save_fn();
						}
					})
					.into(),
				),
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
	// Info,
	Content,
	Console,
}

#[derive(PartialEq, Clone)]
enum LaunchOption {
	Launch,
	LaunchOffline,
	Kill,
}

#[derive(PartialEq, Clone)]
enum MoreOption {
	More,
	Export,
	OpenFolder,
	Delete,
	Custom(usize),
}
