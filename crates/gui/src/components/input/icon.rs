use nitrolaunch::config_crate::template::TemplateConfig;

use crate::{
	components::input::derived_value,
	prelude::*,
	util::{PtrEq, assets::get_instance_icon},
};

static OPTIONS: &[&str] = &[
	"builtin:/icons/fabric.png",
	"builtin:/icons/folia.png",
	"builtin:/icons/forge.png",
	"builtin:/icons/minecraft.png",
	"builtin:/icons/neoforge.png",
	"builtin:/icons/paper.png",
	"builtin:/icons/quilt.png",
	"builtin:/icons/sponge.png",
];

#[derive(PartialEq, Clone)]
pub struct IconSelector {
	pub icon: State<Option<String>>,
	pub parent_configs: PtrEq<[TemplateConfig]>,
}

impl ComponentOwned for IconSelector {
	fn render(self) -> impl IntoElement {
		let theme = use_theme();

		let mut is_open = use_state(|| false);
		let is_hovered = use_state(|| false);

		let derived = derived_value(self.icon.read().as_ref(), &self.parent_configs.0, |x| {
			x.instance.icon.as_ref()
		})
		.cloned();

		let size = 128.0;

		let final_icon = self.icon.read().as_ref().or(derived.as_ref()).cloned();
		let preview = ImageViewer::new(get_instance_icon(final_icon.as_deref()))
			.width(Size::percent(85.0))
			.height(Size::percent(85.0));

		let selector_width = 320.0;
		let cols = 5;
		let option_gap = 8.0;
		let option_size = selector_width / cols as f32 - option_gap;

		let options = std::iter::once(None).chain(OPTIONS.iter().map(Some));
		let options = options.map(|x| x.copied()).map(|x| {
			let is_selected = x == self.icon.read().as_deref();
			let is_derived = x == derived.as_deref() && x.is_some();

			let image = ImageViewer::new(get_instance_icon(x))
				.width(Size::percent(75.0))
				.height(Size::percent(75.0));

			let mut selected = self.icon;

			rect()
				.width(Size::px(option_size))
				.height(Size::px(option_size))
				.item_colorway(&theme, false, is_selected)
				.maybe(is_derived, |this| this.derived_colorway(&theme))
				.corner_radius(theme.round2)
				.center()
				.clickable()
				.on_press(move |_| {
					selected.set(x.map(|x| x.to_string()));
				})
				.child(image)
		});

		let options = ScrollView::new().child(grid(cols, options).gap(8.0));

		let selector = rect()
			.position(Position::new_absolute().left(size + 8.0))
			.width(Size::px(selector_width))
			.height(Size::px(220.0))
			.background(theme.panel)
			.border(theme.border(theme.panel_border))
			.layer(Layer::Overlay)
			.corner_radius(theme.round2)
			.on_pointer_leave(move |_| is_open.set(false))
			.child(options);

		rect()
			.width(Size::px(size))
			.height(Size::px(size))
			.item_colorway(&theme, *is_hovered.read(), false)
			.maybe(derived.is_some(), |this| this.derived_colorway(&theme))
			.hover(is_hovered)
			.corner_radius(theme.round2)
			.center()
			.clickable()
			.on_press(move |_| is_open.toggle())
			.child(preview)
			.maybe(*is_open.read(), |this| this.child(selector))
	}
}
