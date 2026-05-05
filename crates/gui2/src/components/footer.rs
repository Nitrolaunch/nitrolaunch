use crate::prelude::*;

#[derive(PartialEq)]
pub struct Footer;

impl Component for Footer {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let left = rect().height(Size::fill()).width(Size::flex(1.0));

		let center = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.horizontal();

		let right = rect().height(Size::fill()).width(Size::flex(1.0));

		rect()
			.width(Size::fill())
			.height(Size::px(theme.footer_height))
			.horizontal()
			.background(theme.footer)
			.flex()
			.child(left)
			.child(center)
			.child(right)
	}
}
