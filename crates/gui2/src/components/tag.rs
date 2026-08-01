use nitrolaunch::shared::loaders::Loader;

use crate::{prelude::*, util::assets::get_loader_icon};

/// Simple string tag
pub fn text_tag(text: &str, theme: &Theme) -> Rect {
	tag(
		None::<Rect>,
		Some(text),
		theme.fg,
		theme.item,
		theme.item,
		theme,
	)
}

/// Tag for a repository
pub fn repo_tag(repo: &str, compact: bool, back_state: &BackState, theme: &Theme) -> Rect {
	let meta = back_state.repos().get(repo).cloned().unwrap_or_default();

	let name = meta.name.as_deref().unwrap_or(repo);
	let name = if compact { None } else { Some(name) };
	let ico = meta
		.icon
		.as_deref()
		.map(|x| {
			SvgViewer::new(Bytes::from(x.as_bytes().to_vec()))
				.width(Size::px(14.0))
				.height(Size::px(14.0))
				.into_element()
		})
		.unwrap_or(icon("box", 12.0).into_element());
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

	tag(Some(ico), name, fg, bg, bg, theme)
}

/// Tag for a loader
pub fn loader_tag(loader: &Loader, compact: bool, theme: &Theme) -> Rect {
	let name = if compact {
		None
	} else {
		Some(loader.to_string())
	};
	let ico = get_loader_icon(loader)
		.width(Size::px(14.0))
		.height(Size::px(14.0));
	let fg = if compact { theme.fg } else { theme.bg };
	let bg = if compact {
		Color::TRANSPARENT
	} else {
		get_loader_color(loader, theme)
	};

	tag(Some(ico), name.as_deref(), fg, bg, bg, theme)
}

fn get_loader_color(loader: &Loader, theme: &Theme) -> Color {
	match loader {
		Loader::Fabric => Color::from_hex("#d4c9af").unwrap(),
		Loader::Quilt => Color::from_hex("#dc29dd").unwrap(),
		Loader::Forge => Color::from_hex("#505c74").unwrap(),
		Loader::NeoForged => Color::from_hex("#d6732f").unwrap(),
		Loader::Sponge => Color::from_hex("#f8ce0f").unwrap(),
		Loader::SpongeForge => Color::from_hex("#f8ce0f").unwrap(),
		Loader::Paper => Color::from_hex("#fbfbfb").unwrap(),
		Loader::Folia => Color::from_hex("#ff6576").unwrap(),
		_ => theme.fg2,
	}
}

/// Tag element
pub fn tag(
	icon: Option<impl IntoElement>,
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
		.padding(Gaps::new(0.0, theme.gap, 0.0, theme.gap))
		.color(fg)
		.border(theme.border(border))
		.background(bg)
		.corner_radius(theme.round)
		.font_size(12.0)
		.maybe_child(icon)
		.maybe_child(text.map(|x| {
			label()
				.text(x.to_string())
				.font_weight(FontWeight::BOLD)
				.max_lines(1)
		}))
}
