use crate::{prelude::*, util::assets::get_instance_icon};

#[derive(PartialEq, Clone)]
pub struct IconSelector {
	pub icon: State<Option<String>>,
}

impl ComponentOwned for IconSelector {
	fn render(self) -> impl IntoElement {
		let theme = use_theme();

		let mut is_open = use_state(|| false);
		let is_hovered = use_state(|| false);

		let size = 128.0;

		let preview = ImageViewer::new(get_instance_icon(self.icon.read().as_deref()))
			.width(Size::percent(85.0))
			.height(Size::percent(85.0));

		let selector = rect()
			.position(Position::new_absolute().left(size))
			.width(Size::px(256.0))
			.height(Size::px(200.0))
			.background(theme.panel)
			.border(theme.border(theme.panel_border))
			.corner_radius(theme.round2);

		rect()
			.width(Size::px(size))
			.height(Size::px(size))
			.item_colorway(&theme, *is_hovered.read(), false)
			.hover(is_hovered)
			.corner_radius(theme.round2)
			.center()
			.clickable()
            .on_press(move |_| is_open.toggle())
			.child(preview)
			.maybe(*is_open.read(), |this| this.child(selector))
	}
}
