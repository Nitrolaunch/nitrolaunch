use freya::{
	components::{Button, ButtonColorsThemePartialExt, ButtonLayoutThemePartialExt},
	elements::extensions::{EventHandlersExt, StyleExt},
	prelude::{
		Border, BorderAlignment, BorderWidth, ChildrenExt, Color, Component, ContainerExt,
		ContainerSizeExt, ContainerWithContentExt, Content, Cursor, Element, IntoElement, Size,
		rect,
	},
	winit::window::CursorIcon,
};

use crate::theme::Theme;

pub mod footer;
pub mod instance;
pub mod nav;

pub fn button(theme: &Theme) -> Button {
	Button::new()
		.color(theme.fg)
		.background(theme.bg)
		.hover_background(theme.item)
		.border_fill(theme.item_border)
}

pub fn icon_button(icon: &str, theme: &Theme) -> Button {
	let size = 24.0;

	button(theme)
		.background(Color::TRANSPARENT)
		.border_fill(Color::TRANSPARENT)
		.child(crate::icons::icon(icon, 16.0))
		.width(Size::px(size))
		.height(Size::px(size))
		.corner_radius(size / 2.0)
}

pub trait CustomStyles {
	/// Sets full width and height
	fn fill(self) -> Self;

	/// Sets a gap and horizontal layout
	fn cont(self) -> Self;

	/// Sets flex content
	fn flex(self) -> Self;

	/// Sets item border
	fn item_border(self, theme: &Theme) -> Self;
}

impl<T: ContainerSizeExt + StyleExt + ContainerWithContentExt> CustomStyles for T {
	fn fill(self) -> Self {
		self.width(Size::fill()).height(Size::fill())
	}

	fn cont(self) -> Self {
		self.horizontal().spacing(6.0).flex()
	}

	fn flex(self) -> Self {
		self.content(Content::Flex)
	}

	fn item_border(self, theme: &Theme) -> Self {
		self.border(Some(Border {
			fill: theme.item_border.into(),
			width: theme.border.into(),
			alignment: BorderAlignment::Inner,
		}))
	}
}

pub trait CustomEvents {
	/// Sets cursor to pointer on mouse over
	fn clickable(self) -> Self;
}

impl<T: EventHandlersExt> CustomEvents for T {
	fn clickable(self) -> Self {
		self.on_pointer_enter(|_| {
			Cursor::set(CursorIcon::Pointer);
		})
		.on_pointer_leave(|_| {
			Cursor::set(CursorIcon::default());
		})
	}
}

pub fn grid<T: IntoElement + 'static>(cols: u8, items: impl IntoIterator<Item = T>) -> Grid {
	Grid {
		cols,
		gap: 0.0,
		items: items.into_iter().map(|x| x.into_element()).collect(),
	}
}

#[derive(PartialEq)]
pub struct Grid {
	cols: u8,
	gap: f32,
	items: Vec<Element>,
}

impl Grid {
	pub fn gap(mut self, gap: f32) -> Self {
		self.gap = gap;
		self
	}
}

impl Component for Grid {
	fn render(&self) -> impl IntoElement {
		let rows = self.items.chunks(self.cols as usize).map(|items| {
			rect()
				.horizontal()
				.width(Size::fill())
				// .spacing(self.gap)
				.children(items.iter().map(|x| {
					rect()
						.width(Size::percent(100.0 / (self.cols as f32)))
						.child(x.clone())
						.margin(self.gap / 2.0)
						.into_element()
				}))
				.into_element()
		});

		rect()
			.vertical()
			.width(Size::fill())
			.padding(self.gap / 2.0)
			.children(rows)
	}
}

pub fn border_bottom(width: f32, color: impl Into<Color>) -> Border {
	Border {
		fill: color.into(),
		width: BorderWidth {
			bottom: width,
			..Default::default()
		},
		alignment: BorderAlignment::Inner,
	}
}

pub fn border_right(width: f32, color: impl Into<Color>) -> Border {
	Border {
		fill: color.into(),
		width: BorderWidth {
			right: width,
			..Default::default()
		},
		alignment: BorderAlignment::Inner,
	}
}
