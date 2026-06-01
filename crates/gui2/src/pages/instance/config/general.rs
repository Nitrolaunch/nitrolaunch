use std::rc::Rc;

use nitrolaunch::{
	config_crate::{ConfigKind, instance::make_valid_instance_id},
	shared::{
		Side,
		util::{from_string_json, to_string_json},
		versions::VersionPattern,
	},
};

use crate::{
	components::input::{icon::IconSelector, select::Selected, switch::Switch, text::TextInput},
	ops::{
		plugin_results::{FetchLoaderVersions, FetchSupportedLoaders},
		versions::FetchMinecraftVersions,
	},
	pages::instance::config::ConfigState,
	prelude::*,
};

#[derive(PartialEq)]
pub struct GeneralTab {
	pub config_state: ConfigState,
}

impl Component for GeneralTab {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let mut include_snapshots = use_state(|| false);
		let minecraft_versions = use_query(FetchMinecraftVersions::new(
			back_state,
			*include_snapshots.read(),
		));

		let show_id_field =
			self.config_state.is_new && self.config_state.ty != ConfigKind::BaseTemplate;
		let show_name_field = self.config_state.ty != ConfigKind::BaseTemplate;

		let name = use_transform_optional_string(self.config_state.name);

		let mut id = self.config_state.id.clone();
		let mut is_id_dirty = self.config_state.is_id_dirty.clone();
		use_side_effect(move || {
			if !*is_id_dirty.peek() {
				id.set(make_valid_instance_id(&*name.read()));
			}
		});

		let top_right = rect()
			.maybe(show_name_field, |this| {
				this.child(field("Name", &theme, TextInput::new(name)))
			})
			.maybe(show_id_field, |this| {
				this.child(field(
					"ID",
					&theme,
					TextInput::new(self.config_state.id).on_change(move |_| {
						is_id_dirty.set(true);
					}),
				))
			});

		let top = rect()
			.width(Size::fill())
			.height(Size::px(148.0))
			.horizontal()
			.flex()
			.child(
				rect()
					.width(Size::px(148.0))
					.height(Size::fill())
					.center()
					.child(IconSelector {
						icon: self.config_state.icon,
					}),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.padding(Gaps::new(10.0, 10.0, 10.0, 0.0))
					.child(top_right),
			);

		let show_side_field = self.config_state.ty.is_template()
			|| (self.config_state.ty == ConfigKind::Instance && self.config_state.is_new);
		let show_none_option = self.config_state.ty.is_template();
		let side_str = match &*self.config_state.side.read() {
			None => "none",
			Some(Side::Client) => "client",
			Some(Side::Server) => "server",
		};
		let side = self.config_state.side.clone();
		let side_field = InlineSelect::new(
			Selected::Single(side_str.into()),
			Rc::new(move |value| {
				let value = match value.single().as_str() {
					"none" => None,
					"client" => Some(Side::Client),
					"server" => Some(Side::Server),
					_ => unreachable!(),
				};
				side.clone().set(value);
			}),
		)
		.maybe_child(show_none_option, || SelectOption::none())
		.child(SelectOption::new("client", "Client", Some("controller")))
		.child(SelectOption::new("server", "Server", Some("server")));

		let side_field = field("Side", &theme, side_field);

		let minecraft_versions = minecraft_versions
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();
		let minecraft_versions = minecraft_versions
			.into_iter()
			.rev()
			.map(|x| SelectOption::new(&x, &x, None));
		let version = self.config_state.version.clone();
		let version_selector = Dropdown::new(
			Selected::Single(
				self.config_state
					.version
					.read()
					.cloned()
					.unwrap_or("none".into()),
			),
			Rc::new(move |selected| {
				version.clone().set(selected.single_optional());
			}),
		)
		.allow_none()
		.children(minecraft_versions);

		let version_field = rect()
			.cont()
			.child(rect().width(Size::flex(1.0)).child(version_selector))
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::px(theme.input_height))
					.cont()
					.main_align(Alignment::End)
					.cross_align(Alignment::Center)
					.child("Include snapshots")
					.child(Switch {
						enabled: *include_snapshots.read(),
						on_toggle: EventHandler::from(move |_| {
							include_snapshots.toggle();
						}),
					}),
			);
		let version_field = field("Minecraft version", &theme, version_field);

		let loaders_config = LoadersConfig {
			config_state: self.config_state.clone(),
		};

		let main = rect()
			.width(Size::fill())
			.padding(10.0)
			.maybe(show_side_field, |this| this.child(side_field))
			.child(version_field)
			.child(loaders_config);

		rect().expanded().child(top).child(main)
	}
}

#[derive(PartialEq)]
struct LoadersConfig {
	config_state: ConfigState,
}

impl Component for LoadersConfig {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let supported_loaders = use_query(FetchSupportedLoaders::new(back_state.clone()));
		let supported_loaders = supported_loaders
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();
		let minecraft_version = self.config_state.version.read().cloned();
		let client_loader_versions = use_query(FetchLoaderVersions::new(
			back_state.clone(),
			self.config_state
				.client_loader
				.read()
				.cloned()
				.unwrap_or_default(),
			minecraft_version.clone(),
		));
		let server_loader_versions = use_query(FetchLoaderVersions::new(
			back_state,
			self.config_state
				.server_loader
				.read()
				.cloned()
				.unwrap_or_default(),
			minecraft_version.clone(),
		));

		let client_options = supported_loaders
			.iter()
			.filter(|x| x.is_client())
			.map(|x| SelectOption::new(&to_string_json(x), &x.to_string(), None));
		let server_options = supported_loaders
			.iter()
			.filter(|x| x.is_server())
			.map(|x| SelectOption::new(&to_string_json(x), &x.to_string(), None));

		let loader_str = self
			.config_state
			.client_loader
			.read()
			.as_ref()
			.map(|x| to_string_json(x))
			.unwrap_or("none".into());
		let client_loader = self.config_state.client_loader.clone();
		let client_field = InlineSelect::new(
			Selected::Single(loader_str),
			Rc::new(move |selected| {
				let selected = selected.single();
				let selected = from_string_json(&selected).ok();
				client_loader.clone().set(selected);
			}),
		)
		.allow_none()
		.children(client_options);
		let field_name = if self.config_state.ty == ConfigKind::Instance {
			"Loader"
		} else {
			"Client loader"
		};
		let client_field = field(field_name, &theme, client_field);
		let show_client_fields = self.config_state.ty.is_template()
			|| *self.config_state.side.read() == Some(Side::Client);

		let loader_str = self
			.config_state
			.server_loader
			.read()
			.as_ref()
			.map(|x| to_string_json(x))
			.unwrap_or("none".into());
		let server_loader = self.config_state.server_loader.clone();
		let server_field = InlineSelect::new(
			Selected::Single(loader_str),
			Rc::new(move |selected| {
				let selected = selected.single();
				let selected = from_string_json(&selected).ok();
				server_loader.clone().set(selected);
			}),
		)
		.allow_none()
		.children(server_options);
		let field_name = if self.config_state.ty == ConfigKind::Instance {
			"Loader"
		} else {
			"Server loader"
		};
		let server_field = field(field_name, &theme, server_field);
		let show_server_fields = self.config_state.ty.is_template()
			|| *self.config_state.side.read() == Some(Side::Server);

		let client_loader_versions = client_loader_versions
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();
		let options = client_loader_versions
			.into_iter()
			.map(|x| SelectOption::simple(&x));
		let client_version = self.config_state.client_loader_version.clone();
		let client_version_field = Dropdown::new(
			Selected::new_single(
				self.config_state
					.client_loader_version
					.read()
					.optional()
					.map(|x| x.to_string()),
			),
			Rc::new(move |selected| {
				client_version.clone().set(
					selected
						.single_optional()
						.map(|x| VersionPattern::from(&x))
						.unwrap_or_default(),
				);
			}),
		)
		.allow_none()
		.children(options);
		let field_name = if self.config_state.ty == ConfigKind::Instance {
			"Loader version"
		} else {
			"Client loader version"
		};
		let client_version_field = field(field_name, &theme, client_version_field);

		let server_loader_versions = server_loader_versions
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();
		let options = server_loader_versions
			.into_iter()
			.map(|x| SelectOption::simple(&x));
		let server_version = self.config_state.server_loader_version.clone();
		let server_version_field = Dropdown::new(
			Selected::new_single(
				self.config_state
					.server_loader_version
					.read()
					.optional()
					.map(|x| x.to_string()),
			),
			Rc::new(move |selected| {
				server_version.clone().set(
					selected
						.single_optional()
						.map(|x| VersionPattern::from(&x))
						.unwrap_or_default(),
				);
			}),
		)
		.allow_none()
		.children(options);
		let field_name = if self.config_state.ty == ConfigKind::Instance {
			"Loader version"
		} else {
			"Server loader version"
		};
		let server_version_field = field(field_name, &theme, server_version_field);

		rect()
			.width(Size::fill())
			.maybe(show_client_fields, |this| {
				this.child(client_field).child(client_version_field)
			})
			.maybe(show_server_fields, |this| {
				this.child(server_field).child(server_version_field)
			})
	}
}
