use std::borrow::Cow;

use freya::{
	components::SvgViewer,
	elements::extensions::ContainerSizeExt,
	prelude::{Bytes, Size},
};

use crate::prelude::*;

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
	// .maybe(cfg!(debug_assertions), |this| this.color(Color::WHITE))
}

fn icon_impl(icon: &str) -> Cow<'static, [u8]> {
	Icons::get(&format!("icons/{icon}.svg"))
		.map(|x| x.data)
		.unwrap_or_else(|| {
			println!("Unknown icon: {icon}");
			Cow::Owned(Vec::new())
		})
}

pub fn microsoft_icon(theme: &Theme) -> Rect {
	rect()
		.width(Size::px(16.0))
		.height(Size::px(16.0))
		.flex()
		.spacing(1.0)
		.child(
			rect()
				.width(Size::fill())
				.height(Size::flex(1.0))
				.cont()
				.spacing(1.0)
				.child(boxy(theme))
				.child(boxy(theme)),
		)
		.child(
			rect()
				.width(Size::fill())
				.height(Size::flex(1.0))
				.cont()
				.spacing(1.0)
				.child(boxy(theme))
				.child(boxy(theme)),
		)
}

fn boxy(theme: &Theme) -> Rect {
	rect()
		.width(Size::flex(1.0))
		.height(Size::flex(1.0))
		.corner_radius(2.0)
		.background(theme.fg)
}
