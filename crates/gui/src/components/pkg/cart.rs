use crate::{components::pkg::PackageChip, prelude::*};

#[derive(PartialEq)]
pub struct PackageCart;

impl Component for PackageCart {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Cart);

		let header = rect()
			.width(Size::fill())
			.height(Size::px(theme.input_height))
			.horizontal()
			.center()
			.spacing(theme.gap)
			.border(border_bottom(theme.border, theme.panel_border))
			.child(icon("shopping_cart", 16.0))
			.child("Package Cart");

		let package_count = front_state.read().cart().len();
		let packages = ScrollView::new().spacing(theme.gap).children(
			front_state.read().cart().into_iter().map(|x| {
				PackageChip {
					req: x.clone(),
					error: false,
				}
				.into_element()
			}),
		);

		let header_height = theme.input_height;
		let package_count = package_count.min(8);
		let packages_height = package_count as f32 * (theme.input_height + theme.gap);
		let padding_height = theme.gap * 2.0;
		let height = Size::px(header_height + theme.gap + packages_height + padding_height);

		rect()
			.width(Size::px(250.0))
			.height(height)
			.spacing(theme.gap)
			.padding(theme.gap)
			.background(theme.panel)
			.border(theme.border(theme.panel_border))
			.corner_radius(theme.round)
			.overlay_shadow()
			.layer(Layer::Overlay)
			.child(header)
			.child(packages)
	}
}
