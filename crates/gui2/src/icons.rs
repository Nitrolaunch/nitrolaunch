use std::borrow::Cow;

use freya::{
	components::SvgViewer,
	elements::extensions::ContainerSizeExt,
	prelude::{Bytes, Size},
};
use freya_core::{integration::MaybeExt, style::color::Color};

#[derive(rust_embed::RustEmbed)]
#[folder = "./src/assets"]
pub struct Icons;

pub fn icon(icon: &str, size: f32) -> SvgViewer {
	let icon = match icon_impl(icon) {
		Cow::Borrowed(data) => SvgViewer::new((icon, Bytes::from(data))),
		Cow::Owned(data) => SvgViewer::new((icon, Bytes::from(data))),
	};

	icon.width(Size::px(size))
		.height(Size::px(size))
		.parallel(true)
		// Fix pop-in
		.maybe(cfg!(debug_assertions), |this| this.color(Color::WHITE))
}

fn icon_impl(icon: &str) -> Cow<'static, [u8]> {
	Icons::get(&format!("icons/{icon}.svg"))
		.expect("Icon does not exist")
		.data
}
