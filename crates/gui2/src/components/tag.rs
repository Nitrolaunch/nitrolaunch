use crate::prelude::*;

/// Tag for a repository
pub fn repo_tag(repo: &str, compact: bool, back_state: &BackState, theme: &Theme) -> Rect {
	let meta = back_state.repos().get(repo).cloned().unwrap_or_default();

	let name = meta.name.as_deref().unwrap_or(repo);
	let name = if compact { None } else { Some(name) };
	let ico = meta
		.icon
		.as_deref()
		.map(|x| {
			svg(x.as_bytes().to_vec())
				.width(Size::px(12.0))
				.height(Size::px(12.0))
				.into_element()
		})
		.unwrap_or(icon("box", 12.0).into_element());
	let ico = rect()
		.width(Size::px(12.0))
		.height(Size::px(12.0))
		.center()
		.child(ico);
	let fg = meta
		.text_color
		.as_deref()
		.and_then(Color::from_hex)
		.unwrap_or(theme.bg.into());
	let bg = meta
		.color
		.as_deref()
		.and_then(Color::from_hex)
		.unwrap_or(theme.fg2.into());

	tag(ico, name, fg, bg, bg, theme)
}

/// Tag element
pub fn tag(
	icon: impl IntoElement,
	text: Option<&str>,
	fg: impl Into<Color>,
	border: impl Into<Color>,
	bg: impl Into<Color>,
	theme: &Theme,
) -> Rect {
	let fg = fg.into();
	let border = border.into();
	let bg = bg.into();
	rect()
		.height(Size::px(20.0))
		.center()
		.horizontal()
		.spacing(theme.gap)
		.padding(theme.gap)
		.color(fg)
		.border(theme.border(border))
		.background(bg)
		.corner_radius(theme.round)
		.font_size(12.0)
		.child(icon)
		.maybe_child(text)
}
