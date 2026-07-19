use std::sync::Arc;

use freya::prelude::{Border, BorderAlignment, Color, Fill};
use nitrolaunch::shared::util::merge_json_objects;
use serde::{Deserialize, Serialize};

use crate::state::{FrontChannel, use_front_state};

/// Theme for the app
pub struct Theme {
	// Base Colors
	/// Foreground / text color
	pub fg: Color,
	/// Secondary foreground color
	pub fg2: Color,
	/// Tertiary foreground color
	pub fg3: Color,
	/// Background color
	pub bg: Color,
	/// Primary hero color
	pub primary: Color,
	/// Primary hero background color
	pub primary_bg: Color,
	pub secondary: Color,
	pub secondary_bg: Color,
	/// Background color for panels, large segmented sections of UI
	pub panel: Color,
	/// Border color for panels
	pub panel_border: Color,
	pub panel_hover: Color,
	/// Background color for items, smaller UI objects inside panels
	pub item: Color,
	pub item_hover: Color,
	pub item_select: Color,
	pub item_select_border: Color,
	/// Border color for items
	pub item_border: Color,
	/// Highlight background
	pub highlight: Color,
	/// Disabled foreground
	pub disabled: Color,
	/// Template color
	pub template: Color,
	/// Template background color
	pub template_bg: Color,
	/// Warning color
	pub warning: Color,
	/// Error color
	pub error: Color,
	pub error_bg: Color,
	/// Success color
	pub success: Color,
	pub success_bg: Color,
	// Navbar
	pub navbar: Color,
	pub navbar_height: f32,

	// Bottom bar
	pub footer: Color,
	pub footer_height: f32,

	// Side bar
	pub sidebar: Color,
	pub sidebar_width: f32,

	// Other
	/// Font size
	pub font: f32,
	/// Larger font size
	pub font2: f32,
	/// Smallest gap between elements
	pub gap: f32,
	/// Larger gap between elements
	pub gap2: f32,
	/// Even larger gap between elements
	pub gap3: f32,
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
		ThemeDeser::dark().into()
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
	let read = state.read();
	read.subscribe(FrontChannel::Theme);
	read.theme()
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ThemeDeser {
	pub fg: Option<HexColor>,
	pub fg2: Option<HexColor>,
	pub fg3: Option<HexColor>,
	pub bg: Option<HexColor>,
	pub primary: Option<HexColor>,
	pub primary_bg: Option<HexColor>,
	pub secondary: Option<HexColor>,
	pub secondary_bg: Option<HexColor>,
	pub panel: Option<HexColor>,
	pub panel_border: Option<HexColor>,
	pub panel_hover: Option<HexColor>,
	pub item: Option<HexColor>,
	pub item_hover: Option<HexColor>,
	pub item_select: Option<HexColor>,
	pub item_select_border: Option<HexColor>,
	pub item_border: Option<HexColor>,
	pub highlight: Option<HexColor>,
	pub disabled: Option<HexColor>,
	pub template: Option<HexColor>,
	pub template_bg: Option<HexColor>,
	pub warning: Option<HexColor>,
	pub error: Option<HexColor>,
	pub error_bg: Option<HexColor>,
	pub success: Option<HexColor>,
	pub success_bg: Option<HexColor>,
	pub navbar: Option<HexColor>,
	pub navbar_height: Option<f32>,
	pub footer: Option<HexColor>,
	pub footer_height: Option<f32>,
	pub sidebar: Option<HexColor>,
	pub sidebar_width: Option<f32>,
	pub font: Option<f32>,
	pub font2: Option<f32>,
	pub gap: Option<f32>,
	pub gap2: Option<f32>,
	pub gap3: Option<f32>,
	pub border: Option<f32>,
	pub border2: Option<f32>,
	pub round: Option<f32>,
	pub round2: Option<f32>,
	pub input_height: Option<f32>,
}

impl ThemeDeser {
	pub fn dark() -> Self {
		let primary = HexColor("#7ee91b".into());
		let primary_bg = HexColor("#021b1e".into());
		let secondary = HexColor("#d0d0d0".into());
		let secondary_bg = HexColor("#282828".into());
		Self {
			fg: Some(HexColor("#f0f0f0".into())),
			fg2: Some(HexColor("#b5b5b5".into())),
			fg3: Some(HexColor("#757575".into())),
			bg: Some(HexColor("#0e0e0f".into())),
			primary: Some(primary.clone()),
			primary_bg: Some(primary_bg.clone()),
			secondary: Some(secondary.clone()),
			secondary_bg: Some(secondary_bg.clone()),
			panel: Some(HexColor("#111112".into())),
			panel_border: Some(HexColor("#232325".into())),
			panel_hover: Some(HexColor("#191919".into())),
			item: Some(HexColor("#19191b".into())),
			item_border: Some(HexColor("#282829".into())),
			item_hover: Some(HexColor("#1b1b1e".into())),
			item_select: Some(secondary_bg),
			item_select_border: Some(secondary),
			highlight: Some(HexColor("#1c1c1d".into())),
			disabled: Some(HexColor("#656565".into())),
			template: Some(HexColor("#1be9ce".into())),
			template_bg: Some(HexColor("#0d1624".into())),
			warning: Some(HexColor("#e9ca1b".into())),
			error: Some(HexColor("#d40e3d".into())),
			error_bg: Some(HexColor("#2b0e14".into())),
			success: Some(primary),
			success_bg: Some(primary_bg),
			navbar: Some(HexColor("#0c0c0d".into())),
			navbar_height: Some(42.0),
			footer: Some(HexColor("#111112".into())),
			footer_height: Some(48.0),
			sidebar: Some(HexColor("#111112".into())),
			sidebar_width: Some(42.0),
			font: Some(14.0),
			font2: Some(18.0),
			gap: Some(6.0),
			gap2: Some(9.0),
			gap3: Some(14.0),
			border: Some(1.0),
			border2: Some(2.0),
			round: Some(8.0),
			round2: Some(12.0),
			input_height: Some(32.0),
		}
	}

	pub fn light() -> Self {
		let primary = HexColor("#7ee91b".into());
		let primary_bg = HexColor("#f0f7e6".into());
		let secondary = HexColor("#2d2d2d".into());
		let secondary_bg = HexColor("#f0f0f0".into());
		Self {
			fg: Some(HexColor("#0f0f0f".into())),
			fg2: Some(HexColor("#4a4a4a".into())),
			fg3: Some(HexColor("#8a8a8a".into())),
			bg: Some(HexColor("#f1f1f1".into())),
			primary: Some(primary.clone()),
			primary_bg: Some(primary_bg.clone()),
			secondary: Some(secondary.clone()),
			secondary_bg: Some(secondary_bg.clone()),
			panel: Some(HexColor("#e9e9eb".into())),
			panel_border: Some(HexColor("#dcdcde".into())),
			panel_hover: Some(HexColor("#e6e6e6".into())),
			item: Some(HexColor("#e6e6e5".into())),
			item_border: Some(HexColor("#d7d7d7".into())),
			item_hover: Some(HexColor("#e4e4e2".into())),
			item_select: Some(secondary_bg),
			item_select_border: Some(secondary),
			highlight: Some(HexColor("#e3e3e3".into())),
			disabled: Some(HexColor("#9a9a9a".into())),
			template: Some(HexColor("#1be9ce".into())),
			template_bg: Some(HexColor("#e6f9f7".into())),
			warning: Some(HexColor("#e9ca1b".into())),
			error: Some(HexColor("#d40e3d".into())),
			error_bg: Some(HexColor("#fce8f0".into())),
			success: Some(primary),
			success_bg: Some(HexColor("#f0f7e6".into())),
			navbar: Some(HexColor("#f3f3f4".into())),
			navbar_height: Some(42.0),
			footer: Some(HexColor("#eeeef0".into())),
			footer_height: Some(48.0),
			sidebar: Some(HexColor("#eeeef0".into())),
			sidebar_width: Some(42.0),
			font: Some(14.0),
			font2: Some(18.0),
			gap: Some(6.0),
			gap2: Some(9.0),
			gap3: Some(14.0),
			border: Some(1.0),
			border2: Some(2.0),
			round: Some(8.0),
			round2: Some(12.0),
			input_height: Some(32.0),
		}
	}

	pub fn merge(self, other: ThemeDeser) -> Self {
		let this = serde_json::to_value(self).unwrap_or_default();
		let mut this = serde_json::from_value(this).unwrap_or_default();
		let other = serde_json::to_value(other).unwrap_or_default();
		let other = serde_json::from_value(other).unwrap_or_default();
		merge_json_objects(&mut this, other);
		serde_json::from_value(serde_json::Value::Object(this)).unwrap_or_default()
	}
}

impl From<ThemeDeser> for Theme {
	fn from(value: ThemeDeser) -> Self {
		Self {
			fg: value.fg.unwrap_or_default().into(),
			fg2: value.fg2.unwrap_or_default().into(),
			fg3: value.fg3.unwrap_or_default().into(),
			bg: value.bg.unwrap_or_default().into(),
			primary: value.primary.unwrap_or_default().into(),
			primary_bg: value.primary_bg.unwrap_or_default().into(),
			secondary: value.secondary.unwrap_or_default().into(),
			secondary_bg: value.secondary_bg.unwrap_or_default().into(),
			panel: value.panel.unwrap_or_default().into(),
			panel_border: value.panel_border.unwrap_or_default().into(),
			panel_hover: value.panel_hover.unwrap_or_default().into(),
			item: value.item.unwrap_or_default().into(),
			item_hover: value.item_hover.unwrap_or_default().into(),
			item_select: value.item_select.unwrap_or_default().into(),
			item_select_border: value.item_select_border.unwrap_or_default().into(),
			item_border: value.item_border.unwrap_or_default().into(),
			highlight: value.highlight.unwrap_or_default().into(),
			disabled: value.disabled.unwrap_or_default().into(),
			template: value.template.unwrap_or_default().into(),
			template_bg: value.template_bg.unwrap_or_default().into(),
			warning: value.warning.unwrap_or_default().into(),
			error: value.error.unwrap_or_default().into(),
			error_bg: value.error_bg.unwrap_or_default().into(),
			success: value.success.unwrap_or_default().into(),
			success_bg: value.success_bg.unwrap_or_default().into(),
			navbar: value.navbar.unwrap_or_default().into(),
			navbar_height: value.navbar_height.unwrap_or(42.0),
			footer: value.footer.unwrap_or_default().into(),
			footer_height: value.footer_height.unwrap_or(48.0),
			sidebar: value.sidebar.unwrap_or_default().into(),
			sidebar_width: value.sidebar_width.unwrap_or(42.0),
			font: value.font.unwrap_or(14.0),
			font2: value.font2.unwrap_or(18.0),
			gap: value.gap.unwrap_or(6.0),
			gap2: value.gap2.unwrap_or(9.0),
			gap3: value.gap3.unwrap_or(14.0),
			border: value.border.unwrap_or(1.0),
			border2: value.border2.unwrap_or(2.0),
			round: value.round.unwrap_or(8.0),
			round2: value.round2.unwrap_or(12.0),
			input_height: value.input_height.unwrap_or(32.0),
		}
	}
}

/// Color with serde support so the whole serde feature doesn't have to be enabled on Freya
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct HexColor(String);

impl HexColor {
	pub fn to_color(self) -> Color {
		self.into()
	}
}

impl From<HexColor> for Color {
	fn from(value: HexColor) -> Self {
		Color::from_hex(&value.0).unwrap_or_default()
	}
}

impl From<HexColor> for Fill {
	fn from(value: HexColor) -> Self {
		value.to_color().into()
	}
}

#[derive(Clone, Copy, PartialEq)]
pub struct Colorway {
	pub fg: Color,
	pub bg: Color,
	pub border: Color,
}

impl Colorway {
	pub fn new(fg: impl Into<Color>, bg: impl Into<Color>, border: impl Into<Color>) -> Self {
		Self {
			fg: fg.into(),
			bg: bg.into(),
			border: border.into(),
		}
	}
}
