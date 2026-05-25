use crate::prelude::*;

#[derive(PartialEq)]
pub struct Switch {
	pub enabled: bool,
	pub on_toggle: EventHandler<()>,
}

impl Component for Switch {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let border_width = 2.0;
		let position = if self.enabled { 16.0 } else { 0.0 };
		let color = if self.enabled {
			theme.primary.into()
		} else {
			theme.disabled.into()
		};

		let on_toggle = self.on_toggle.clone();

		rect()
			.width(Size::px(32.0 + border_width * 2.0))
			.height(Size::px(16.0 + border_width * 2.0))
			.border(Border {
				width: border_width.into(),
				fill: color,
				alignment: BorderAlignment::Inner,
			})
			.corner_radius(8.0 + border_width)
			.padding(2.0 + border_width)
			.on_press(move |_| on_toggle.call(()))
            .clickable()
			.child(
				rect()
					.width(Size::px(16.0 - border_width * 2.0))
					.height(Size::px(16.0 - border_width * 2.0))
					.corner_radius(8.0 - border_width)
					.background(color)
					.position(Position::new_absolute().left(position)),
			)
	}
}
