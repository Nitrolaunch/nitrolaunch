use crate::prelude::*;
use nitrolaunch::config_crate::ConfigKind;
use nitrolaunch::core::util::versions::MinecraftVersion;
use nitrolaunch::shared::Side;
use nitrolaunch::shared::loaders::Loader;

#[derive(PartialEq)]
pub struct InstanceListItem {
	info: InstanceItemInfo,
	is_selected: bool,
}

impl InstanceListItem {
	pub fn new(info: InstanceItemInfo, is_selected: bool) -> Self {
		Self { info, is_selected }
	}
}

impl Component for InstanceListItem {
	fn render(&self) -> impl IntoElement {
		rect()
			.width(Size::fill())
			.height(Size::px(60.0))
			.center()
			.rounded()
			.background((50, 50, 50))
			.child(&*self.info.id)
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
