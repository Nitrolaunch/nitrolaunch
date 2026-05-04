use std::sync::Arc;

use freya::{prelude::Color, radio::use_radio};
use serde::Deserialize;

use crate::state::AppChannel;

/// Theme for the app
#[derive(Deserialize)]
pub struct Theme {
	// Base Colors

	/// Foreground / text color
	pub fg: HexColor,
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
	/// Border color for items
	pub item_border: HexColor,
	/// Disabled foreground
	pub disabled: HexColor,

	// Navbar

	pub navbar: HexColor,
	pub navbar_height: f32,

	// Other

	/// Border width
	pub border: f32,
	/// Smaller border radius
	pub round: f32,
	/// Larger border radius
	pub round2: f32,
}

impl Theme {
	pub fn dark() -> Self {
		Self {
			fg: HexColor(0xfff6f6f6),
			bg: HexColor(0xff0c0c0c),
			primary: HexColor(0xff7ee91b),
			primary_bg: HexColor(0xff051d1d),
			panel: HexColor(0xff111111),
			panel_border: HexColor(0xff2b2b2b),
			item: HexColor(0xff1a1a1a),
			item_border: HexColor(0xff2b2b2b),
			disabled: HexColor(0xff777777),
			navbar: HexColor(0xff0c0c0c),
			navbar_height: 32.0,
			border: 2.0,
			round: 6.0,
			round2: 12.0,
		}
	}
}

/// Gets the theme
pub fn use_theme() -> Arc<Theme> {
	let state = use_radio(AppChannel::Theme);
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
