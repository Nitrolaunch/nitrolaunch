use crate::ops::instance::InstanceItemInfo;
use crate::prelude::*;
use crate::util::assets::get_instance_icon;
use nitrolaunch::shared::Side;

pub mod running_instances;

#[derive(PartialEq)]
pub struct InstanceListItem {
	info: InstanceItemInfo,
	selected: State<Option<InstanceItemInfo>>,
}

impl InstanceListItem {
	pub fn new(info: InstanceItemInfo, selected: State<Option<InstanceItemInfo>>) -> Self {
		Self { info, selected }
	}
}

impl Component for InstanceListItem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let is_hovered = use_state(|| false);

		let is_selected = self
			.selected
			.read()
			.as_ref()
			.is_some_and(|x| x == &self.info);

		let mut selected = self.selected.clone();

		let name = if let Some(name) = &self.info.name {
			name
		} else {
			&self.info.id
		};

		let inst_icon = get_instance_icon(self.info.icon.as_deref());

		let name_weight = if is_selected {
			FontWeight::BOLD
		} else {
			FontWeight::NORMAL
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
					.font_weight(name_weight)
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
			theme.primary
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
					.text_overflow(TextOverflow::Clip)
					.overflow(Overflow::Clip)
					.child(version),
			);

		let info = self.info.clone();

		rect()
			.width(Size::fill())
			.height(Size::px(110.0))
			.flex()
			.corner_radius(theme.round2)
			.item_colorway(&theme, *is_hovered.read(), is_selected)
			.on_press(move |_| selected.set(Some(info.clone())))
			.clickable()
			.hover(is_hovered)
			.child(top)
			.child(bottom)
	}
}
