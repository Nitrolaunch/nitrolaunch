use crate::prelude::*;

#[derive(PartialEq)]
pub struct PageButtons {
	pub page: usize,
	pub total_pages: usize,
	pub on_set: EventHandler<usize>,
}

impl Component for PageButtons {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let buttons = (-2..=2).map(|offset| {
			let cont = rect().width(Size::px(32.0)).height(Size::fill()).center();
			if self.page as i32 + offset < 0 || self.page as i32 + offset > self.total_pages as i32
			{
				let button = button(&theme)
					.width(Size::px(32.0))
					.height(Size::px(32.0))
					.padding(theme.gap)
					.corner_radius(theme.round)
					.enabled(false)
					.child(
						rect()
							.width(Size::px(4.0))
							.height(Size::px(4.0))
							.corner_radius(2.0)
							.background(theme.disabled),
					);

				cont.child(button).into_element()
			} else {
				let i = self.page as i32 + offset;
				let is_selected = offset == 0;
				let on_set = self.on_set.clone();

				let button = button(&theme)
					.width(Size::px(32.0))
					.height(Size::px(32.0))
					.padding(theme.gap)
					.corner_radius(theme.round)
					.maybe(is_selected, |this| this.background(theme.highlight))
					.maybe(!is_selected, |this| this.color(theme.disabled))
					.on_press(move |_| on_set.call(i as usize))
					.child(format!("{}", i + 1));

				cont.child(button).into_element()
			}
		});

		rect()
			.height(Size::px(32.0))
			.cont()
			.padding(theme.gap)
			.spacing(theme.gap)
			.children(buttons)
	}
}
