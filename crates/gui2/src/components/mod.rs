use std::sync::Arc;

use gpui::{
	App, Context, Div, InteractiveElement, IntoElement, RenderOnce, SharedString,
	StatefulInteractiveElement, Styled, div, px,
};
use gpui_component::{ActiveTheme, h_flex, tooltip::Tooltip};

pub mod instance;
pub mod nav;

/// Centered horizontal flex
pub fn center() -> Div {
	h_flex().justify_center().content_center()
}

/// Flex-grow block (like a section of a grid) with min-height set to 0 to prevent growing to content
pub fn sect() -> Div {
	div().flex_1().min_h_0()
}

/// Gapped flexbox
pub fn cont() -> Div {
	center().gap(px(6.0))
}

pub trait CustomStyles {
	/// Sets flex-grow
	fn grow(self, size: f32) -> Self;

	/// Sets flex, justify-center, and items-center
	fn center(self) -> Self;

	/// Sets flex, justify-start, and items-center
	fn start(self) -> Self;

	/// Sets flex, justify-end, and items-center
	fn end(self) -> Self;

	/// Sets border to consistent style
	fn bordered<T: 'static>(self, cx: &Context<T>) -> Self;

	/// Sets border radius to consistent style
	fn round(self) -> Self;
}

impl<T: Styled> CustomStyles for T {
	fn grow(mut self, size: f32) -> Self {
		self.style().flex_grow = Some(size);
		self
	}

	fn center(self) -> Self {
		self.flex().justify_center().items_center()
	}

	fn start(self) -> Self {
		self.flex().justify_start().items_center()
	}

	fn end(self) -> Self {
		self.flex().justify_end().items_center()
	}

	fn bordered<V: 'static>(self, cx: &Context<V>) -> Self {
		self.border_2().border_color(cx.theme().border)
	}

	fn round(self) -> Self {
		self.rounded_md()
	}
}

pub trait CustomStylesInteractive {
	/// Adds a tooltip
	fn tip(self, tip: &str) -> Self;
}

impl<T: Styled + StatefulInteractiveElement> CustomStylesInteractive for T {
	fn tip(self, tip: &str) -> Self {
		let tip = SharedString::from(tip);
		self.tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
	}
}

pub fn show<T: IntoElement, F: FnOnce() -> T>(show: bool, elem: F) -> impl IntoElement {
	if show {
		elem().into_any_element()
	} else {
		div().into_any_element()
	}
}

pub fn show_multi<T: IntoElement, F: FnOnce() -> Vec<T>>(show: bool, elem: F) -> Vec<T> {
	if show { elem() } else { Vec::new() }
}
