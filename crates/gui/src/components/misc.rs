use nitrolaunch::shared::util::open_link;

use crate::prelude::*;

static DISCORD_ICON: &str = "https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Fi.pinimg.com%2Foriginals%2Fb5%2Fd4%2Fce%2Fb5d4ce10a744861ffd3314d20d116976.jpg&f=1&nofb=1&ipt=6d62e58133af814dc726e3ec8aceb847e0e652df00aa08798e8916e72a539b63";
static GITHUB_ICON: &str = "https://external-content.duckduckgo.com/iu/?u=https%3A%2F%2Fgithub.githubassets.com%2Fassets%2FGitHub-Mark-ea2971cee799.png&f=1&nofb=1&ipt=8a9527d1ac1d5044e7e73d75826bddeb5964a81490c0bd4ba9acce5c13431a0b";

pub fn socials(theme: &Theme) -> impl IntoElement {
	rect()
		.width(Size::fill())
		.cont()
		.padding(theme.gap)
		.child(
			segment(
				button(theme)
					.child(
						img(DISCORD_ICON)
							.width(Size::px(32.0))
							.height(Size::px(32.0))
							.corner_radius(theme.round),
					)
					.on_press(move |_| {
						let _ = open_link("https://discord.gg/25fhkjeTvW");
					}),
				1.0,
			)
			.center(),
		)
		.child(
			segment(
				button(theme).child(icon("globe", 32.0)).on_press(move |_| {
					let _ = open_link("https://nitrolaunch.github.io/");
				}),
				1.0,
			)
			.center(),
		)
		.child(
			segment(
				button(theme)
					.child(
						img(GITHUB_ICON)
							.width(Size::px(32.0))
							.height(Size::px(32.0))
							.corner_radius(theme.round),
					)
					.on_press(move |_| {
						let _ = open_link("https://github.com/Nitrolaunch/nitrolaunch");
					}),
				1.0,
			)
			.center(),
		)
}

pub fn progress_bar(theme: &Theme, progress: f32) -> Rect {
	rect()
		.width(Size::fill())
		.height(Size::px(8.0))
		.corner_radius(4.0)
		.panel_colorway(theme, false, false)
		.child(
			rect()
				.width(Size::percent(progress * 100.0))
				.height(Size::fill())
				.corner_radius(4.0)
				.background(theme.primary),
		)
}

pub fn status_panel(text: &str, color: Color, theme: &Theme) -> Rect {
	rect()
		.padding(theme.gap3)
		.color(color)
		.border(theme.border(color))
		.corner_radius(theme.round)
		.child(label().text(text.to_string()).font_size(16.0))
}

/// Usually for indicating a number of items, like a notification count
pub fn number_indicator(num: usize, theme: &Theme) -> Rect {
	rect()
		.height(Size::px(20.0))
		.min_width(Size::px(20.0))
		.padding(Gaps::new(0.0, theme.gap, 0.0, theme.gap))
		.center()
		.corner_radius(theme.round)
		.background(theme.item)
		.font_size(theme.font0)
		.font_weight(FontWeight::BOLD)
		.color(theme.fg3)
		.child(num.to_string())
}
