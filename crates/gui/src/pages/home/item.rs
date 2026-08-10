use crate::ops::instance::InstanceItemInfo;
use crate::pages::config::ConfiguredItem;
use crate::prelude::*;
use crate::routing::Page;
use crate::state::ModalType;
use crate::util::assets::get_instance_icon;
use nitrolaunch::config_crate::ConfigKind;
use nitrolaunch::shared::Side;

#[derive(PartialEq)]
pub struct InstanceListItem {
	info: InstanceItemInfo,
	selected: State<Option<InstanceItemInfo>>,
	is_add_placeholder: bool,
}

impl InstanceListItem {
	pub fn new(info: InstanceItemInfo, selected: State<Option<InstanceItemInfo>>) -> Self {
		Self {
			info,
			selected,
			is_add_placeholder: false,
		}
	}

	pub fn add_placeholder(ty: ConfigKind, selected: State<Option<InstanceItemInfo>>) -> Self {
		Self {
			info: InstanceItemInfo {
				id: "Create new".into(),
				ty,
				name: None,
				icon: Some("builtin:/icons/plus.svg".into()),
				side: None,
				version: None,
				loader: None,
				source_plugin: None,
				is_editable: true,
				is_deletable: false,
			},
			selected,
			is_add_placeholder: true,
		}
	}
}

impl Component for InstanceListItem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();

		let is_hovered = use_state(|| false);

		let is_selected = self
			.selected
			.read()
			.as_ref()
			.is_some_and(|x| x == &self.info);

		let mut selected = self.selected;

		let name = if let Some(name) = &self.info.name {
			name
		} else {
			&self.info.id
		};

		let inst_icon = if self.is_add_placeholder {
			icon("plus", 26.0).into_element()
		} else if self.info.icon.is_none() {
			icon("box", 32.0).into_element()
		} else {
			let inst_icon = get_instance_icon(self.info.icon.as_deref());
			ImageViewer::new(inst_icon)
				.width(Size::percent(60.0))
				.height(Size::percent(60.0))
				.into_element()
		};

		let top = rect()
			.cont()
			.width(Size::fill())
			.height(Size::px(72.0))
			.child(
				rect()
					.width(Size::px(72.0))
					.height(Size::fill())
					.center()
					.child(inst_icon),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.horizontal()
					.cross_align(Alignment::Center)
					.font_weight(FontWeight::BOLD)
					.child(name.as_str()),
			);

		let side = if let Some(side) = &self.info.side {
			let ico = match side {
				Side::Client => "controller",
				Side::Server => "server",
			};
			rect()
				.cont()
				.child(icon(ico, 16.0))
				.child(side.to_string_pretty())
		} else {
			rect()
		};

		let loader = if let Some(loader) = &self.info.loader {
			rect()
				.cont()
				.child(icon("box_highlight", 16.0))
				.child(loader.to_string())
		} else {
			rect()
		};

		let version = if let Some(version) = &self.info.version {
			rect()
				.cont()
				.child(icon("tag", 16.0))
				.child(clip_text(&version.to_string()).width(Size::auto()))
		} else {
			rect()
		};

		let bottom_color = if is_selected {
			theme.item_select_border
		} else {
			theme.fg3
		};

		let bottom = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.horizontal()
			.flex()
			.color(bottom_color)
			.font_weight(FontWeight::BOLD)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.cont()
					.center()
					.text_overflow(TextOverflow::Clip)
					.overflow(Overflow::Clip)
					.child(side),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.cont()
					.center()
					.text_overflow(TextOverflow::Clip)
					.overflow(Overflow::Clip)
					.child(loader),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.cont()
					.center()
					.child(version),
			);

		let info = self.info.clone();
		let is_add_placeholder = self.is_add_placeholder;

		let front_state2 = front_state.clone();
		let on_click = move |_| {
			// Add placeholder
			if is_add_placeholder {
				front_state2
					.write()
					.set_modal(Some(ModalType::Configuration(ConfiguredItem {
						id: None,
						ty: info.ty,
						is_new: true,
					})));

				return;
			}

			// Double click
			if is_selected {
				match info.ty {
					ConfigKind::Instance => {
						front_state2
							.write()
							.navigate(Page::Instance(info.id.clone()));
					}
					ConfigKind::Template | ConfigKind::BaseTemplate => {
						front_state2
							.write()
							.set_modal(Some(ModalType::Configuration(info.get_config_item())));
					}
				}
			} else {
				selected.set(Some(info.clone()))
			}
		};

		let plugin_indicator = self.info.source_plugin.as_ref().map(|x| {
			let size = 28.0;
			rect()
				.width(Size::px(size))
				.height(Size::px(size))
				.position(Position::new_absolute().top(-size / 2.0).right(-size / 2.0))
				.center()
				.color(theme.secondary)
				.border(theme.border(theme.secondary))
				.background(theme.secondary_bg)
				.corner_radius(theme.round)
				.tip(&front_state, &format!("From the {x} plugin"))
				.child(icon("jigsaw", 16.0))
		});

		rect()
			.width(Size::fill())
			.height(Size::px(110.0))
			.flex()
			.corner_radius(theme.round2)
			.panel_colorway(&theme, *is_hovered.read(), is_selected)
			.on_press(on_click)
			.clickable()
			.hover(is_hovered)
			.child(top)
			.child(bottom)
			.maybe_child(plugin_indicator)
	}
}
