use crate::pages::home::SelectedLocation;
use crate::prelude::*;
use crate::util::assets::get_instance_icon;
use nitrolaunch::config_crate::ConfigKind;
use nitrolaunch::core::util::versions::MinecraftVersion;
use nitrolaunch::shared::Side;
use nitrolaunch::shared::loaders::Loader;

#[derive(PartialEq)]
pub struct InstanceListItem {
	info: InstanceItemInfo,
	selected: State<Option<SelectedLocation>>,
}

impl InstanceListItem {
	pub fn new(info: InstanceItemInfo, selected: State<Option<SelectedLocation>>) -> Self {
		Self { info, selected }
	}
}

impl Component for InstanceListItem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let location = SelectedLocation::from_item(&self.info);
		let is_selected = self
			.selected
			.read()
			.as_ref()
			.is_some_and(|x| x.is_selected(&self.info));

		let mut selected = self.selected.clone();

		let colors = if is_selected {
			(theme.primary, theme.primary, theme.primary_bg)
		} else {
			(theme.fg, theme.item_border, theme.item)
		};

		let name = if let Some(name) = &self.info.name {
			name
		} else {
			&self.info.id
		};

		let inst_icon = get_instance_icon(self.info.icon.as_deref());

		let top = rect()
			.cont()
			.width(Size::fill())
			.height(Size::px(72.0))
			.border(Some(border_bottom(theme.border, colors.1)))
			.child(
				rect()
					.width(Size::px(72.0))
					.height(Size::fill())
					.center()
					.child(
						ImageViewer::new(inst_icon)
							.width(Size::percent(60.0))
							.height(Size::percent(60.0)),
					),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.horizontal()
					.cross_align(Alignment::Center)
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
				.child(icon("box", 16.0))
				.child(loader.to_string())
		} else {
			rect()
		};

		let version = if let Some(version) = &self.info.version {
			rect()
				.cont()
				.child(icon("tag", 16.0))
				.child(version.to_string())
		} else {
			rect()
		};

		let bottom_color = if is_selected {
			colors.0
		} else {
			theme.disabled
		};

		let bottom = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.horizontal()
			.flex()
			.color(bottom_color)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.cont()
					.center()
					.border(Some(border_right(theme.border, colors.1)))
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
					.border(Some(border_right(theme.border, colors.1)))
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
					.text_overflow(TextOverflow::Clip)
					.overflow(Overflow::Clip)
					.child(version),
			);

		rect()
			.width(Size::fill())
			.height(Size::px(100.0))
			.flex()
			.corner_radius(theme.round2)
			.color(colors.0)
			.background(colors.2)
			.border(Some(Border {
				fill: colors.1.into(),
				width: theme.border.into(),
				alignment: BorderAlignment::Inner,
			}))
			.on_press(move |_| selected.set(Some(location.clone())))
			.clickable()
			.child(top)
			.child(bottom)
	}
}

/// Simple info about an instance or template
#[derive(Clone, PartialEq)]
pub struct InstanceItemInfo {
	pub id: String,
	pub ty: ConfigKind,
	pub name: Option<String>,
	pub icon: Option<String>,
	pub side: Option<Side>,
	pub version: Option<MinecraftVersion>,
	pub loader: Option<Loader>,
}
