use freya::prelude::{
	ChildrenExt, Component, ContainerExt, ContainerSizeExt, ContainerWithContentExt, IntoElement,
	Size, rect,
};

pub mod instance;
pub mod nav;

pub trait CustomStyles {
	/// Sets full width and height
	fn fill(self) -> Self;
}

impl<T: ContainerSizeExt> CustomStyles for T {
	fn fill(self) -> Self {
		self.width(Size::fill()).height(Size::fill())
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
				.spacing(self.gap)
				.children(items.iter().map(|x| {
					rect()
						.width(Size::percent(100.0 / (self.cols as f32)))
						.child(x.render())
						.into_element()
				}))
				.into_element()
		});

		rect()
			.vertical()
			.width(Size::fill())
			.spacing(self.gap)
			.padding(self.gap)
			.children(rows)
	}
}
