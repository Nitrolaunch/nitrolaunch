use std::{rc::Rc, sync::Arc};

use crate::{
	components::input::{select::Selected, text::transparent_text_input},
	prelude::*,
	util::PtrEq,
};

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

		let ty = use_state::<Option<MessageType>>(|| None);
		let input = use_state(|| String::new());

		use_side_effect({
			move || {
				let ty = ty.read().clone();

				if let Some(contents) = &*contents.read() {
					let mut all_indexes = Vec::new();
					all_indexes.push(0);

					for (i, ch) in contents.char_indices() {
						if ch == '\n' {
							all_indexes.push(i + 1);
						}
					}

					let mut indexes = Vec::new();
					let mut uppercase_buf = String::new();
					for (i, start) in all_indexes.iter().copied().enumerate() {
						let end = all_indexes.get(i + 1).copied().unwrap_or(contents.len());
						let line = contents.get(start..end).unwrap_or_default().trim_end();

						if line_matches_ty(line, ty.as_ref(), &mut uppercase_buf) {
							indexes.push(start);
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

		let theme2 = theme.clone();
		let contents = match self.console.contents() {
			Some(contents) => {
				let indexes = line_indexes.clone();
				VirtualScrollView::new_with_data(PtrEq(contents.clone()), move |item, contents| {
					let indexes_read = indexes.read();
					let line = if item.index < indexes_read.len() {
						let start = indexes_read[item.index];
						let end = if item.index + 1 < indexes_read.len() {
							indexes_read[item.index + 1]
						} else {
							contents.0.len()
						};

						contents.0.get(start..end).unwrap_or_default().trim_end()
					} else {
						""
					};

					format_line(line, &theme2)
				})
				.width(Size::fill())
				.height(Size::flex(1.0))
				.item_size(24.0)
				.length(*line_count.read())
				.into_element()
			}
			None => {
				let pholder = if self.console.is_loading() {
					"Loading..."
				} else {
					"No output available"
				};

				rect()
					.width(Size::fill())
					.height(Size::flex(1.0))
					.child(placeholder(pholder, &theme))
					.into_element()
			}
		};

		let input_fn = self.console.input_fn();
		let mut input2 = input.clone();
		let contents = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.background(theme.bg)
			.border(theme.border(theme.panel_border))
			.padding(theme.gap2)
			.corner_radius(theme.round2)
			.flex()
			.child(contents)
			.maybe(
				input_fn.is_some() && self.console.get_log_file().is_none(),
				|this| {
					this.child(
						rect()
							.width(Size::fill())
							.height(Size::px(theme.input_height))
							.border(border_top(theme.border, theme.panel_border))
							.corner_radius(CornerRadius::new(0.0, 0.0, theme.round2, theme.round2))
							.padding(theme.gap)
							.cont()
							.child(transparent_text_input(input, &theme).on_submit(move |s| {
								let result = input_fn.as_ref().unwrap()(s);
								if result {
									input2.set(String::new());
								}
							})),
					)
				},
			);

		let ty_selector = Dropdown::new(
			Selected::Single(ty.read().clone()),
			Rc::new(move |new| {
				ty.clone().set(new.single());
			}),
		)
		.header_width(Size::auto())
		.child(SelectOption::new(
			None,
			"All Messages",
			Some("speech_bubble"),
		))
		.child(SelectOption::new(
			Some(MessageType::Error),
			"Errors",
			Some("error"),
		))
		.child(SelectOption::new(
			Some(MessageType::Warning),
			"Warnings",
			Some("warning"),
		))
		.child(SelectOption::new(
			Some(MessageType::Info),
			"Info",
			Some("info"),
		));

		let console2 = self.console.clone();
		let file_selector = Dropdown::new(
			Selected::Single(self.console.get_log_file()),
			Rc::new(move |new| {
				console2.set_log_file(new.single());
			}),
		)
		.header_width(Size::auto())
		.options_width(180.0)
		.child(SelectOption::new(None, "Current Output", Some("text")))
		.children(
			self.console
				.get_log_files()
				.map(|x| SelectOption::new(Some(x.clone()), x, Some("text"))),
		);

		let header = rect()
			.width(Size::fill())
			.height(Size::px(theme.input_height))
			.cont()
			.child(segment(ty_selector, 1.0))
			.child(segment(rect(), 1.0))
			.child(
				segment(file_selector, 1.0)
					.horizontal()
					.main_align(Alignment::End),
			);

		rect()
			.expanded()
			.flex()
			.padding(theme.gap2)
			.spacing(theme.gap)
			.child(header)
			.child(contents)
	}
}

fn format_line(line: &str, theme: &Theme) -> Element {
	let (left, ty, right, ty_color) = if let Some((left, right)) = line.split_once("ERROR") {
		(left, "ERROR", right, theme.error)
	} else if let Some((left, right)) = line.split_once("WARN") {
		(left, "WARN", right, theme.warning)
	} else if let Some((left, right)) = line.split_once("INFO") {
		(left, "INFO", right, theme.fg)
	} else {
		return clip_text(line).height(Size::px(24.0)).into_element();
	};

	rect()
		.width(Size::fill())
		.height(Size::px(24.0))
		.horizontal()
		.child(label().text(left.to_string()).max_lines(1))
		.child(label().text(ty).color(ty_color))
		.child(
			label()
				.text(right.to_string())
				.max_lines(1)
				.color(theme.fg3),
		)
		.into_element()
}

pub trait ConsoleImpl: PartialEq + Clone + 'static {
	fn contents(&self) -> Option<Arc<str>>;

	fn is_loading(&self) -> bool {
		false
	}

	fn get_log_files(&self) -> impl Iterator<Item = &String>;

	fn get_log_file(&self) -> Option<String>;

	fn set_log_file(&self, file: Option<String>);

	fn input_fn(&self) -> Option<impl Fn(String) -> bool + 'static> {
		None::<Box<dyn Fn(String) -> bool>>
	}
}

#[derive(PartialEq, Clone)]
enum MessageType {
	Info,
	Warning,
	Error,
}

fn line_matches_ty(line: &str, ty: Option<&MessageType>, uppercase_buf: &mut String) -> bool {
	match ty {
		None => true,
		Some(ty) => line_type(line, uppercase_buf).as_ref() == Some(ty),
	}
}

fn line_type(line: &str, uppercase_buf: &mut String) -> Option<MessageType> {
	uppercase_buf.clear();
	if line.len() > 32 {
		uppercase_buf.push_str(&line[..32]);
	} else {
		uppercase_buf.push_str(line);
	}
	uppercase_buf.make_ascii_uppercase();

	if uppercase_buf.contains("ERROR") || uppercase_buf.contains("[ERR]") {
		Some(MessageType::Error)
	} else if uppercase_buf.contains("WARN") {
		Some(MessageType::Warning)
	} else if uppercase_buf.contains("INFO") {
		Some(MessageType::Info)
	} else {
		None
	}
}
