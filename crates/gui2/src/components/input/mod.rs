use crate::prelude::*;

pub mod control;
pub mod icon;
pub mod select;
pub mod switch;
pub mod tabs;
pub mod text;

/// A label above a configuration field
pub fn field_label(text: &str, theme: &Theme) -> impl IntoElement {
	rect()
		.width(Size::fill())
		.horizontal()
		.main_align(Alignment::Start)
		.cross_align(Alignment::End)
		.font_size(13)
		.font_weight(FontWeight::BOLD)
		.color(theme.fg3)
		.padding(Gaps::new(0.0, 0.0, 5.0, 3.0))
		.child(text)
}

/// Configuration field with a label
pub fn field(label: &str, theme: &Theme, field: impl IntoElement) -> impl IntoElement {
	rect()
		.width(Size::fill())
		.margin(Gaps::new(0.0, 0.0, 12.0, 0.0))
		.child(field_label(label, theme))
		.child(field)
}
