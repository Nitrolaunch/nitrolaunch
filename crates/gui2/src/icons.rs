use std::borrow::Cow;

use freya::{
	elements::{
		extensions::ContainerSizeExt,
		svg::{Svg, svg},
	},
	prelude::Size,
};

#[derive(rust_embed::RustEmbed)]
#[folder = "./src/assets"]
pub struct Icons;

pub fn icon(icon: &str, size: f32) -> Svg {
	let icon = match icon_impl(icon) {
		Cow::Borrowed(data) => svg(data),
		Cow::Owned(data) => svg(data),
	};

	icon.width(Size::px(size)).height(Size::px(size))
}

fn icon_impl(icon: &str) -> Cow<'static, [u8]> {
	Icons::get(&format!("icons/{icon}.svg")).unwrap().data
}
