use crate::prelude::*;

#[derive(PartialEq)]
pub struct Gallery {
	pub items: Vec<ImageSource>,
	pub columns: u8,
}

impl Component for Gallery {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let items = self.items.iter().map(|x| {
			rect()
				.width(Size::fill())
				.height(Size::px(180.0))
				.corner_radius(theme.round2)
				.shiny_border(&theme)
				.child(
					ImageViewer::new(x.clone())
						.expanded()
						.aspect_ratio(AspectRatio::Max)
						.image_cover(ImageCover::Center)
						.corner_radius(theme.round2),
				)
		});
		let grid = grid(self.columns, items).gap(theme.gap2);
		ScrollView::new().expanded().child(grid)
	}
}
