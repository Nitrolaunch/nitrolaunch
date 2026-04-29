use gpui::prelude::*;
use gpui::*;
use nitrolaunch::config_crate::ConfigKind;

pub struct InstanceListItem {
	id: String,
	ty: ConfigKind,
}

impl InstanceListItem {
	pub fn new(id: String, ty: ConfigKind, _: &Window, _: &mut Context<Self>) -> Self {
		Self { id, ty }
	}
}

impl Render for InstanceListItem {
	fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		gpui_rsx::rsx! {
			<div size_full flex justify_center items_center>{self.id.clone()}</div>
		}
	}
}

/// Simple info about an instance or template
pub struct InstanceItemInfo {
	pub id: String,
	pub ty: ConfigKind,
}
