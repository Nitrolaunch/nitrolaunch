use std::sync::Arc;

use freya::prelude::Color;
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
	/// Background color for panels, large segmented sections of UI
	pub panel: HexColor,
	/// Border color for panels
	pub panel_border: HexColor,
	/// Background color for items, smaller UI objects inside panels
	pub item: HexColor,
	pub item_hover: HexColor,
	pub item_select: HexColor,
	pub item_select_border: HexColor,
	/// Border color for items
	pub item_border: HexColor,
	/// Disabled foreground
	pub disabled: HexColor,

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
	/// Border width
	pub border: f32,
	/// Smaller border radius
	pub round: f32,
	/// Larger border radius
	pub round2: f32,
	/// Height for inputs
	pub input_height: f32,
}

impl Theme {
	pub fn dark() -> Self {
		Self {
			fg: HexColor(0xfff0f0f0),
			fg2: HexColor(0xffb5b5b5),
			fg3: HexColor(0xff757575),
			bg: HexColor(0xff0c0c0d),
			primary: HexColor(0xff7ee91b),
			primary_bg: HexColor(0xff021b1e),
			panel: HexColor(0xff131315),
			panel_border: HexColor(0xff2b2b2c),
			item: HexColor(0xff1a1a1b),
			item_border: HexColor(0xff2b2b2c),
			item_hover: HexColor(0xff1d1d1e),
			item_select: HexColor(0xff202021),
			item_select_border: HexColor(0xff282829),
			disabled: HexColor(0xff656565),
			navbar: HexColor(0xff0c0c0d),
			navbar_height: 42.0,
			footer: HexColor(0xff111112),
			footer_height: 48.0,
			sidebar: HexColor(0xff111112),
			sidebar_width: 42.0,
			border: 2.0,
			round: 6.0,
			round2: 12.0,
			input_height: 32.0,
		}
	}

	pub fn dark_minimal() -> Self {
		let dark = Self::dark();

		Self {
			item: dark.panel,
			item_border: HexColor(0xff212122),
			item_hover: dark.item,
			item_select: dark.primary_bg,
			item_select_border: dark.primary,
			border: 1.0,
			..dark
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

impl From<HexColor> for Color {
	fn from(value: HexColor) -> Self {
		Color::new(value.0)
	}
}
