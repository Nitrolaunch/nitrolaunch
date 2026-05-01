use crate::prelude::*;

pub mod router;

#[derive(PartialEq)]
pub struct NavBar;

impl Component for NavBar {
	fn render(&self) -> impl IntoElement {
		rect()
			.width(Size::fill())
			.height(Size::px(32.0))
			.background((15, 15, 15))
	}
}
