use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;

use crate::state::AppState;

pub struct HomePage {
	app_state: AppState,
}

impl HomePage {
	pub fn new(app_state: AppState, _: &Window, _: &mut Context<Self>) -> Self {
		Self { app_state }
	}
}

impl Render for HomePage {
	fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		gpui_rsx::rsx! {
			<div size_full flex justify_center items_center>{Button::new("foo").label("Button!")}</div>
		}
	}
}
