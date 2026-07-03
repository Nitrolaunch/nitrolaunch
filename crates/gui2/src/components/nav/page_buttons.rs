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
				cont.into_element()
			} else {
				let i = self.page as i32 + offset;
				let is_selected = offset == 0;
				let on_set = self.on_set.clone();
				let bg = if is_selected {
					theme.item_select
				} else {
					theme.bg
				};

				let button = rect()
					.width(Size::px(32.0))
					.height(Size::px(32.0))
					.center()
					.padding(theme.gap)
					.corner_radius(theme.round)
					.background(bg)
					.on_press(move |_| on_set.call(i as usize))
					.clickable()
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
