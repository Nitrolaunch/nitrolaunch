use std::rc::Rc;

use freya::query::UseMutation;
use itertools::Itertools;
use nitrolaunch::shared::{loaders::Loader, util::open_link};

use crate::{
	components::input::select::Selected,
	ops::{
		ConditionalQuery, ToastedMutation,
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
					let default = Vec::new();
					let local_plugins = local_plugins.read();
					let local_plugins = local_plugins.state();
					let is_local_too = local_plugins
						.ok()
						.unwrap_or(&default)
						.iter()
						.any(|p| p.id == x.id);
					if *is_remote.peek() && is_local_too {
						return None;
					}

					Some(
						PluginItem {
							info: NotEq(x),
							is_remote: *is_remote.peek(),
							install_mutation: NotEq(install_mutation.clone()),
							uninstall_mutation: NotEq(uninstall_mutation.clone()),
							enable_mutation: NotEq(enable_mutation.clone()),
							disable_mutation: NotEq(disable_mutation.clone()),
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
	install_mutation: NotEq<UseMutation<ToastedMutation<InstallPlugin>>>,
	uninstall_mutation: NotEq<UseMutation<ToastedMutation<UninstallPlugin>>>,
	enable_mutation: NotEq<UseMutation<ToastedMutation<EnablePlugin>>>,
	disable_mutation: NotEq<UseMutation<ToastedMutation<DisablePlugin>>>,
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

		let id = self.info.0.id.clone();
		let install_mutation = self.install_mutation.clone();
		let install = move |_: Event<PressEventData>| {
			install_mutation.0.mutate((id.clone(), None));
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

		let id = info.id.clone();
		let version_selector = Dropdown::new(
			Selected::Single(info.version.clone()),
			Rc::new(move |selected| {
				let version = selected.single().clone();
				install_mutation.0.mutate((id.clone(), version));
			}),
		)
		.hide_arrow()
		.header_width(Size::auto())
		.child(SelectOption::new(None, "Any", Some("tag")))
		.children(
			versions
				.read()
				.state()
				.ok()
				.cloned()
				.unwrap_or_default()
				.into_iter()
				.map(|v| SelectOption::new(Some(v.clone()), &v, Some("download"))),
		)
		.on_open_change(move |is_open| {
			if is_open {
				fetch_versions.set(true)
			}
		})
		.loading(!versions.read().state().is_ok())
		.maybe_child(
			!versions.read().state().is_ok() && info.version.is_some(),
			|| {
				SelectOption::new(
					info.version.clone(),
					info.version.as_deref().unwrap(),
					Some("tag"),
				)
			},
		);

		let id = info.id.clone();
		let documentation = info.meta.documentation.clone();
		let enable_mutation = self.enable_mutation.clone();
		let disable_mutation = self.disable_mutation.clone();
		let uninstall_mutation = self.uninstall_mutation.clone();
		let more_dropdown = Dropdown::new(
			Selected::Single(MoreDropdown::More),
			Rc::new(move |selected| match selected.single() {
				MoreDropdown::More => {}
				MoreDropdown::Documentation => {
					if let Some(doc) = &documentation {
						let _ = open_link(doc);
					}
				}
				MoreDropdown::Enable => {
					enable_mutation.0.mutate(id.clone());
				}
				MoreDropdown::Disable => {
					disable_mutation.0.mutate(id.clone());
				}
				MoreDropdown::Uninstall => {
					uninstall_mutation.0.mutate(id.clone());
				}
			}),
		)
		.custom_header(SelectOption::new(MoreDropdown::More, "", Some("elipsis")))
		.header_width(Size::auto())
		.hide_arrow()
		.options_width(180.0)
		.maybe_child(self.info.0.meta.documentation.is_some(), || {
			SelectOption::new(MoreDropdown::Documentation, "Documentation", Some("book"))
		})
		.maybe_child(!self.is_remote && !info.enabled, || {
			SelectOption::new(MoreDropdown::Enable, "Enable", Some("check"))
		})
		.maybe_child(!self.is_remote && info.enabled, || {
			SelectOption::new(MoreDropdown::Disable, "Disable", Some("delete"))
		})
		.maybe_child(!self.is_remote, || {
			SelectOption::new(MoreDropdown::Uninstall, "Uninstall", Some("trash"))
		});

		let controls = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.cont()
			.spacing(theme.gap2)
			.cross_align(Alignment::Center)
			.main_align(Alignment::End)
			.padding(Gaps::new(0.0, theme.gap2, 0.0, 0.0))
			.child(version_selector)
			.maybe(self.is_remote, |this| {
				this.child(icon_text_button("download", "Install", &theme).on_press(install))
			})
			.maybe(!self.is_remote, |this| this.child(more_dropdown));

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

#[derive(PartialEq, Clone)]
enum MoreDropdown {
	More,
	Documentation,
	Enable,
	Disable,
	Uninstall,
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
		"shortcut" => "popout",
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
