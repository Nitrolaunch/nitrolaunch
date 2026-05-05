use crate::{pages::home::HomePage, prelude::*, routing::Page};

#[derive(PartialEq)]
pub struct Router {}

impl Router {
	pub fn new() -> Self {
		Self {}
	}
}

impl Component for Router {
	fn render(&self) -> impl IntoElement {
		let state = use_radio(AppChannel::Route);

		let child = match state.read().navigator.route() {
			Page::Home => HomePage.into_element(),
			Page::Packages => rect().into_element(),
			Page::Plugins => rect().into_element(),
		};

		rect().width(Size::fill()).height(Size::fill()).child(child)
	}
}
