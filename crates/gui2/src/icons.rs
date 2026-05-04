use std::borrow::Cow;

use freya::elements::svg::{Svg, svg};

#[derive(rust_embed::RustEmbed)]
#[folder = "./src/assets"]
pub struct Icons;

pub fn icon(icon: &str) -> Svg {
	match icon_impl(icon) {
		Cow::Borrowed(data) => svg(data),
		Cow::Owned(data) => svg(data),
	}
}

fn icon_impl(icon: &str) -> Cow<'static, [u8]> {
	Icons::get(&format!("icons/{icon}.svg")).unwrap().data
}
