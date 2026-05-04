use freya::{
	components::{Button, ButtonColorsThemePartialExt},
	elements::extensions::{EventHandlersExt, LayoutExt, StyleExt},
	prelude::{
		Border, BorderAlignment, ChildrenExt, Component, ContainerExt, ContainerSizeExt,
		ContainerWithContentExt, Cursor, IntoElement, Size, rect,
	},
	winit::window::CursorIcon,
};

use crate::theme::Theme;

pub mod instance;
pub mod nav;

pub fn button(theme: &Theme) -> Button {
	Button::new()
		.color(theme.fg)
		.background(theme.bg)
		.hover_background(theme.panel)
		.border_fill(theme.item_border)
}

pub trait CustomStyles {
	/// Sets full width and height
	fn fill(self) -> Self;

	/// Sets a gap and horizontal layout
	fn cont(self) -> Self;

	/// Sets item border
	fn item_border(self, theme: &Theme) -> Self;
}

impl<T: ContainerSizeExt + StyleExt + ContainerWithContentExt> CustomStyles for T {
	fn fill(self) -> Self {
		self.width(Size::fill()).height(Size::fill())
	}

	fn cont(self) -> Self {
		self.horizontal().spacing(6.0)
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

pub fn grid<T: Component + 'static>(cols: u8, items: impl IntoIterator<Item = T>) -> Grid<T> {
	Grid {
		cols,
		gap: 0.0,
		items: items.into_iter().collect(),
	}
}

#[derive(PartialEq)]
pub struct Grid<T: Component + 'static> {
	cols: u8,
	gap: f32,
	items: Vec<T>,
}

impl<T: Component + 'static> Grid<T> {
	pub fn gap(mut self, gap: f32) -> Self {
		self.gap = gap;
		self
	}
}

impl<T: Component + 'static> Component for Grid<T> {
	fn render(&self) -> impl IntoElement {
		let rows = self.items.chunks(self.cols as usize).map(|items| {
			rect()
				.horizontal()
				.width(Size::fill())
				// .spacing(self.gap)
				.children(items.iter().map(|x| {
					rect()
						.width(Size::percent(100.0 / (self.cols as f32)))
						.child(x.render())
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
