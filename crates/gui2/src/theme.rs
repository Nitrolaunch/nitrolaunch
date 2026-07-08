use std::sync::Arc;

use freya::prelude::{Border, BorderAlignment, Color, Fill};
use serde::Deserialize;

use crate::state::use_front_state;

/// Theme for the app
#[derive(Deserialize)]
pub struct Theme {
	// Base Colors
	/// Foreground / text color
	pub fg: HexColor,
	/// Secondary foreground color
	pub fg2: HexColor,
	/// Tertiary foreground color
	pub fg3: HexColor,
	/// Background color
	pub bg: HexColor,
	/// Primary hero color
	pub primary: HexColor,
	/// Primary hero background color
	pub primary_bg: HexColor,
	pub secondary: HexColor,
	pub secondary_bg: HexColor,
	/// Background color for panels, large segmented sections of UI
	pub panel: HexColor,
	/// Border color for panels
	pub panel_border: HexColor,
	pub panel_hover: HexColor,
	/// Background color for items, smaller UI objects inside panels
	pub item: HexColor,
	pub item_hover: HexColor,
	pub item_select: HexColor,
	pub item_select_border: HexColor,
	/// Border color for items
	pub item_border: HexColor,
	/// Highlight background
	pub highlight: HexColor,
	/// Disabled foreground
	pub disabled: HexColor,
	/// Template color
	pub template: HexColor,
	/// Template background color
	pub template_bg: HexColor,
	/// Warning color
	pub warning: HexColor,
	/// Error color
	pub error: HexColor,
	pub error_bg: HexColor,
	/// Success color
	pub success: HexColor,
	pub success_bg: HexColor,

	// Navbar
	pub navbar: HexColor,
	pub navbar_height: f32,

	// Bottom bar
	pub footer: HexColor,
	pub footer_height: f32,

	// Side bar
	pub sidebar: HexColor,
	pub sidebar_width: f32,

	// Other
	/// Smallest gap between elements
	pub gap: f32,
	/// Larger gap between elements
	pub gap2: f32,
	/// Border width
	pub border: f32,
	/// Larger border width
	pub border2: f32,
	/// Smaller border radius
	pub round: f32,
	/// Larger border radius
	pub round2: f32,
	/// Height for inputs
	pub input_height: f32,
}

impl Theme {
	pub fn dark() -> Self {
		let primary = HexColor(0xff7ee91b);
		let primary_bg = HexColor(0xff021b1e);
		let secondary = HexColor(0xffd0d0d0);
		let secondary_bg = HexColor(0xff282828);
		Self {
			fg: HexColor(0xfff0f0f0),
			fg2: HexColor(0xffb5b5b5),
			fg3: HexColor(0xff757575),
			bg: HexColor(0xff0e0e0f),
			primary,
			primary_bg,
			secondary,
			secondary_bg,
			panel: HexColor(0xff111112),
			panel_border: HexColor(0xff232325),
			panel_hover: HexColor(0xff191919),
			item: HexColor(0xff19191b),
			item_border: HexColor(0xff282829),
			item_hover: HexColor(0xff1b1b1e),
			item_select: secondary_bg,
			item_select_border: secondary,
			highlight: HexColor(0xff1c1c1d),
			disabled: HexColor(0xff656565),
			template: HexColor(0xff1be9ce),
			template_bg: HexColor(0xff0d1624),
			warning: HexColor(0xffe9ca1b),
			error: HexColor(0xffd40e3d),
			error_bg: HexColor(0xff2b0e14),
			success: primary,
			success_bg: primary_bg,
			navbar: HexColor(0xff0c0c0d),
			navbar_height: 42.0,
			footer: HexColor(0xff111112),
			footer_height: 48.0,
			sidebar: HexColor(0xff111112),
			sidebar_width: 42.0,
			gap: 6.0,
			gap2: 9.0,
			border: 1.0,
			border2: 2.0,
			round: 8.0,
			round2: 12.0,
			input_height: 32.0,
		}
	}

	pub fn border(&self, color: impl Into<Color>) -> Border {
		Border {
			width: self.border.into(),
			fill: color.into(),
			alignment: BorderAlignment::Inner,
		}
	}

	pub fn border2(&self, color: impl Into<Color>) -> Border {
		Border {
			width: self.border2.into(),
			fill: color.into(),
			alignment: BorderAlignment::Inner,
		}
	}
}

/// Gets the theme
pub fn use_theme() -> Arc<Theme> {
	let state = use_front_state();
	state.read().theme()
}

/// Color with serde support so the whole serde feature doesn't have to be enabled on Freya
#[derive(Clone, Copy, Deserialize)]
pub struct HexColor(u32);

impl HexColor {
	pub fn to_color(self) -> Color {
		self.into()
	}
}

impl From<HexColor> for Color {
	fn from(value: HexColor) -> Self {
		Color::new(value.0)
	}
}

impl From<HexColor> for Fill {
	fn from(value: HexColor) -> Self {
		value.to_color().into()
	}
}
