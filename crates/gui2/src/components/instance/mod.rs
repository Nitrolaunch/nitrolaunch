use crate::pages::home::SelectedLocation;
use crate::prelude::*;
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

		rect()
			.width(Size::fill())
			.height(Size::px(90.0))
			.center()
			.corner_radius(theme.round2)
			.color(colors.0)
			.background(colors.2)
			.border(Some(Border {
				fill: colors.1.into(),
				width: theme.border.into(),
				alignment: BorderAlignment::Inner,
			}))
			.child(&*self.info.id)
			.on_press(move |_| selected.set(Some(location.clone())))
			.clickable()
	}
}

/// Simple info about an instance or template
#[derive(Clone, PartialEq)]
pub struct InstanceItemInfo {
	pub id: String,
	pub ty: ConfigKind,
	pub name: Option<String>,
	pub side: Option<Side>,
	pub version: Option<MinecraftVersion>,
	pub loader: Option<Loader>,
}
