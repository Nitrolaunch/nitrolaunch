use std::sync::Arc;

use crate::{components::input::text::transparent_text_input, prelude::*, util::PtrEq};

#[derive(PartialEq)]
pub struct Console<C: ConsoleImpl> {
	pub console: C,
}

impl<C: ConsoleImpl> Component for Console<C> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		// Used for reactivity
		let contents = use_reactive(&self.console.contents());
		let mut line_indexes = use_state::<Vec<usize>>(|| Vec::new());
		let mut line_count = use_state::<usize>(|| 0);

		let input = use_state(|| String::new());

		use_side_effect({
			move || {
				if let Some(contents) = &*contents.read() {
					let mut indexes = Vec::new();
					indexes.push(0);

					for (i, ch) in contents.char_indices() {
						if ch == '\n' {
							indexes.push(i + 1);
						}
					}

					let count = indexes.len();
					line_indexes.set(indexes);
					line_count.set(count);
				} else {
					line_indexes.set(Vec::new());
					line_count.set(0);
				}
			}
		});

		let contents = match self.console.contents() {
			Some(contents) => {
				let indexes = line_indexes.clone();
				VirtualScrollView::new_with_data(PtrEq(contents.clone()), move |i, contents| {
					let indexes_read = indexes.read();
					let line = if i < indexes_read.len() {
						let start = indexes_read[i];
						let end = if i + 1 < indexes_read.len() {
							indexes_read[i + 1]
						} else {
							contents.0.len()
						};

						contents.0.get(start..end).unwrap_or_default().trim_end()
					} else {
						""
					};

					label()
						.text(line.to_string())
						.height(Size::px(24.0))
						.into_element()
				})
				.width(Size::fill())
				.height(Size::flex(1.0))
				.item_size(24.0)
				.length(*line_count.read())
				.into_element()
			}
			None => rect()
				.width(Size::fill())
				.height(Size::flex(1.0))
				.child(placeholder(
					"No output available. Is the instance running?",
					&theme,
				))
				.into_element(),
		};

		let contents = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.background(theme.bg)
			.border(theme.border(theme.panel_border))
			.padding(theme.gap)
			.corner_radius(theme.round2)
			.flex()
			.child(contents)
			.maybe(self.console.input_fn().is_some(), |this| {
				this.child(
					rect()
						.width(Size::fill())
						.height(Size::px(theme.input_height))
						.border(border_top(theme.border, theme.panel_border))
						.corner_radius(theme.round2)
						.padding(theme.gap)
						.cont()
						.child(
							transparent_text_input(input, &theme)
								.on_submit(self.console.input_fn().unwrap()),
						),
				)
			});

		let header = rect()
			.width(Size::fill())
			.height(Size::px(theme.input_height));

		rect()
			.expanded()
			.flex()
			.padding(theme.gap2)
			.spacing(theme.gap)
			.child(header)
			.child(contents)
	}
}

pub trait ConsoleImpl: PartialEq + 'static {
	fn contents(&self) -> Option<Arc<str>>;

	fn input_fn(&self) -> Option<impl Fn(String) + 'static> {
		None::<Box<dyn Fn(String)>>
	}
}
