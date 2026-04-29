use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::scroll::ScrollableElement;
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
			<div w_full h_32 flex justify_center items_center bg={cx.theme().secondary} overflow_scrollbar>
				// {self.id.clone()}
				<div h={px(500.0)} bg={cx.theme().primary} w_full>{"Scroll me"}</div>
			</div>
		}
	}
}

/// Simple info about an instance or template
pub struct InstanceItemInfo {
	pub id: String,
	pub ty: ConfigKind,
}
