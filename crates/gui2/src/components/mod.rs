use gpui::{App, Context, Div, IntoElement, RenderOnce, Styled, div, px};
use gpui_component::{ActiveTheme, h_flex};

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

pub fn show<T: IntoElement>() -> Show<T> {
	Show {
		elem: None,
		show: false,
	}
}

#[derive(IntoElement)]
pub struct Show<T: IntoElement + 'static> {
	elem: Option<T>,
	show: bool,
}

impl<T: IntoElement> Show<T> {
	pub fn child(mut self, e: T) -> Self {
		self.elem = Some(e);
		self
	}

	pub fn show(mut self, show: bool) -> Self {
		self.show = show;
		self
	}
}

impl<T: IntoElement + 'static> RenderOnce for Show<T> {
	fn render(self, _: &mut gpui::Window, _: &mut App) -> impl IntoElement {
		if let Some(elem) = self.elem {
			if self.show {
				elem.into_any_element()
			} else {
				div().into_any_element()
			}
		} else {
			div().into_any_element()
		}
	}
}
