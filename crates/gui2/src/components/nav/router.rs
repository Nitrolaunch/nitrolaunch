use crate::{
	pages::{home::HomePage, instance::InstancePage, package::browse::BrowsePackagesPage},
	prelude::*,
	routing::Page,
};

#[derive(PartialEq)]
pub struct Router {}

impl Router {
	pub fn new() -> Self {
		Self {}
	}
}

impl Component for Router {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Route);

		let child = match front_state.read().route() {
			Page::Home => HomePage.into_element(),
			Page::Packages => BrowsePackagesPage.into_element(),
			Page::Instance(id) => InstancePage { id: id.clone() }.into_element(),
		};

		rect().width(Size::fill()).height(Size::fill()).child(child)
	}
}
