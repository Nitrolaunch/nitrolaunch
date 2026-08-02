use std::rc::Rc;

use itertools::Itertools;
use nitrolaunch::shared::{loaders::Loader, util::open_link};

use crate::{
	components::input::{select::Selected, switch::Switch},
	ops::{
		ConditionalQuery,
		plugins::{
			DisablePlugin, EnablePlugin, FetchLocalPlugins, FetchPluginVersions,
			FetchRemotePlugins, InstallPlugin, PluginInfo, UninstallPlugin,
		},
	},
	prelude::*,
	util::assets::get_loader_icon,
};

#[derive(PartialEq)]
pub struct PluginsPage;

impl Component for PluginsPage {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let local_plugins = use_query(Query::new((), FetchLocalPlugins::new(back_state.clone())));
		let remote_plugins = use_query(Query::new((), FetchRemotePlugins::new(back_state.clone())));

		let is_remote = use_state(|| false);

		let location_selector = InlineSelect::new(
			Selected::Single(*is_remote.read()),
			Rc::new(move |selected| {
				is_remote.clone().set(selected.single());
			}),
		)
		.child(SelectOption::new(false, "Installed", Some("folder")))
		.child(SelectOption::new(true, "Available", Some("globe")));

		let header = rect()
			.width(Size::fill())
			.cont()
			.child(segment(location_selector, 1.0))
			.child(segment(rect(), 1.0))
			.child(segment(
				label()
					.width(Size::fill())
					.text_align(TextAlign::End)
					.color(theme.fg3)
					.text("You may need to restart the app for changes to take effect."),
				1.0,
			));

		let plugins = if *is_remote.read() {
			remote_plugins
				.read()
				.state()
				.ok()
				.cloned()
				.unwrap_or_default()
		} else {
			local_plugins
				.read()
				.state()
				.ok()
				.cloned()
				.unwrap_or_default()
		};
		let plugins = ScrollView::new().expanded().spacing(theme.gap).children(
			plugins
				.into_iter()
				.sorted_by_cached_key(|x| x.id.clone())
				.filter_map(|x| {
					if *is_remote.peek() {
						if local_plugins
							.read()
							.state()
							.ok()
							.cloned()
							.unwrap_or_default()
							.iter()
							.any(|p| p.id == x.id)
						{
							return None;
						}
					}

					Some(
						PluginItem {
							info: NotEq(x),
							is_remote: *is_remote.peek(),
						}
						.into_element(),
					)
				}),
		);
		let contents = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.child(plugins);

		rect()
			.expanded()
			.flex()
			.spacing(theme.gap2)
			.padding(theme.gap2)
			.child(header)
			.child(contents)
	}
}

#[derive(PartialEq)]
struct PluginItem {
	info: NotEq<PluginInfo>,
	is_remote: bool,
}

impl Component for PluginItem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();

		let mut fetch_versions = use_state(|| false);
		let id = self.info.0.id.clone();
		let versions = use_query(ConditionalQuery::new(
			FetchPluginVersions::new(back_state.clone()).toast(
				&back_state,
				None,
				"Failed to fetch plugin versions",
			),
			*fetch_versions.read(),
			|| id.clone(),
		));

		let install_mutation =
			use_mutation(Mutation::new(InstallPlugin::new(back_state.clone()).toast(
				&back_state,
				Some("Plugin installed"),
				"Failed to install plugin",
			)));
		let uninstall_mutation = use_mutation(Mutation::new(
			UninstallPlugin::new(back_state.clone()).toast(
				&back_state,
				Some("Plugin uninstalled"),
				"Failed to uninstall plugin",
			),
		));
		let enable_mutation =
			use_mutation(Mutation::new(EnablePlugin::new(back_state.clone()).toast(
				&back_state,
				Some("Plugin enabled"),
				"Failed to enable plugin",
			)));
		let disable_mutation =
			use_mutation(Mutation::new(DisablePlugin::new(back_state.clone()).toast(
				&back_state,
				Some("Plugin disabled"),
				"Failed to disable plugin",
			)));

		let id = self.info.0.id.clone();
		let install = move |_: Event<PressEventData>| {
			install_mutation.mutate((id.clone(), None));
		};

		let id = self.info.0.id.clone();
		let uninstall = move |_: Event<PressEventData>| {
			uninstall_mutation.mutate(id.clone());
		};

		let info = &self.info.0;
		let ico = get_plugin_icon(&info.id);
		let (ico_fg, ico_bg, ico_tip) = if info.is_official {
			(theme.success, theme.success_bg, "Official")
		} else {
			(theme.fg, theme.bg, "Unofficial")
		};
		let ico = rect()
			.width(Size::px(48.0))
			.height(Size::px(48.0))
			.center()
			.child(
				rect()
					.width(Size::px(32.0))
					.height(Size::px(32.0))
					.center()
					.background(ico_bg)
					.corner_radius(theme.round)
					.tip(&front_state, ico_tip)
					.color(ico_fg)
					.child(ico),
			);
		let name = info.meta.name.clone().unwrap_or_else(|| info.id.clone());

		let enabled = info.enabled;
		let id = self.info.0.id.clone();
		let enable_switch = if self.is_remote {
			None
		} else {
			let tip = if enabled {
				"Click to disable"
			} else {
				"Click to enable"
			};
			Some(rect().tip(&front_state, tip).child(Switch {
				enabled,
				on_toggle: EventHandler::new(move |_| {
					if enabled {
						disable_mutation.mutate(id.clone());
					} else {
						enable_mutation.mutate(id.clone());
					}
				}),
			}))
		};

		let id = info.id.clone();
		let version_selector = Dropdown::new(
			Selected::Single(info.version.clone()),
			Rc::new(move |selected| {
				let version = selected.single().clone();
				install_mutation.mutate((id.clone(), version));
			}),
		)
		.child(SelectOption::new(None, "Any", None))
		.children(
			versions
				.read()
				.state()
				.ok()
				.cloned()
				.unwrap_or_default()
				.into_iter()
				.map(|v| SelectOption::simple_or_none(Some(v))),
		)
		.on_open_change(move |is_open| {
			if is_open {
				fetch_versions.set(true)
			}
		})
		.loading(!versions.read().state().is_ok())
		.maybe_child(!versions.read().state().is_ok(), || {
			SelectOption::simple_or_none(info.version.clone())
		});

		let documentation = info.meta.documentation.clone();
		let controls = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.cont()
			.spacing(theme.gap2)
			.cross_align(Alignment::Center)
			.main_align(Alignment::End)
			.padding(Gaps::new(0.0, theme.gap2, 0.0, 0.0))
			.maybe_child(enable_switch)
			.maybe(self.info.0.meta.documentation.is_some(), |this| {
				this.child(rect().tip(&front_state, "Documentation").child(
					icon_button("book", &theme).on_press(move |_| {
						let _ = open_link(documentation.as_deref().unwrap());
					}),
				))
			})
			.maybe(self.is_remote, |this| {
				this.child(
					rect().tip(&front_state, "Install").child(
						button(&theme)
							.on_press(install)
							.child(icon("download", 16.0)),
					),
				)
			})
			.maybe(!self.is_remote, |this| {
				this.child(
					rect()
						.tip(&front_state, "Uninstall")
						.child(icon_button("trash", &theme).on_press(uninstall)),
				)
			})
			.child(rect().width(Size::px(84.0)).child(version_selector));

		let header = rect()
			.width(Size::fill())
			.height(Size::px(48.0))
			.cont()
			.child(ico)
			.child(
				segment(label().text(name), 1.0)
					.height(Size::fill())
					.main_align(Alignment::Center),
			)
			.child(controls);

		let description = info.meta.description.clone().unwrap_or_default();

		rect()
			.width(Size::fill())
			.border(theme.border(theme.panel_border))
			.corner_radius(theme.round)
			.child(header)
			.child(
				rect()
					.width(Size::fill())
					.padding(Gaps::new(0.0, theme.gap2, theme.gap2, theme.gap2))
					.color(theme.fg2)
					.child(description),
			)
	}
}

pub fn get_plugin_icon(plugin: &str) -> impl IntoElement {
	let image_icon = match plugin {
		"fabric_quilt" => Some(get_loader_icon(&Loader::Fabric)),
		"paper" => Some(get_loader_icon(&Loader::Paper)),
		"sponge" => Some(get_loader_icon(&Loader::Sponge)),
		_ => None,
	};

	if let Some(img) = image_icon {
		return img
			.width(Size::px(16.0))
			.height(Size::px(16.0))
			.into_element();
	}

	let icon_name = match plugin {
		"args" => "text",
		"automate" | "options" | "config_split" => "gear",
		"backup" => "download",
		"better_jsons" => "curly_braces",
		"cleanup" => "trash",
		"completions" => "font",
		"custom_files" => "folder",
		"docs" => "book",
		"doctor" => "heart",
		"extra_versions" => "curly_braces",
		"gamepad" => "controller",
		"glfw_fix" => "heart",
		"guardian" => "helmet",
		"lang" => "language",
		"multiply" => "honeycomb",
		"octane" => "lightning",
		"share" => "popout",
		"server_restart" => "refresh",
		"shorcut" => "popout",
		"skin_stealer" => "multiple_users",
		"stats" => "graph",
		"themes" => "palette",
		"webtools" => "globe",
		"weld" => "link",
		_ if plugin.contains("transfer") => "cycle",
		_ => "box",
	};

	icon(icon_name, 16.0).into_element()
}
