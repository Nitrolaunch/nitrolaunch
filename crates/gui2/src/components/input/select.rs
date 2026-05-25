use std::rc::Rc;

use freya::query::Captured;

use crate::prelude::*;

#[derive(PartialEq)]
pub struct InlineSelect {
	options: Vec<SelectOption>,
	selected: Selected,
	on_select: Captured<Rc<dyn Fn(Selected)>>,
	align_end: bool,
}

impl InlineSelect {
	pub fn new(selected: Selected, on_select: Rc<dyn Fn(Selected)>) -> Self {
		Self {
			options: Vec::new(),
			selected,
			on_select: Captured(on_select),
			align_end: false,
		}
	}

	pub fn child(mut self, child: SelectOption) -> Self {
		self.options.push(child);
		self
	}

	pub fn children(mut self, children: impl IntoIterator<Item = SelectOption>) -> Self {
		for child in children {
			self = self.child(child);
		}
		self
	}

	pub fn align_end(mut self) -> Self {
		self.align_end = true;
		self
	}
}

impl Component for InlineSelect {
	fn render(&self) -> impl IntoElement {
		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_select: Captured<Rc<dyn Fn(String)>> =
			Captured(Rc::new(move |option| match &selected {
				Selected::Single(..) => {
					(upper_on_select)(Selected::Single(option));
				}
				Selected::Multi(options) => {
					let mut options = options.clone();
					options.push(option);
					(upper_on_select)(Selected::Multi(options));
				}
			}));

		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_deselect: Captured<Rc<dyn Fn(String)>> =
			Captured(Rc::new(move |option| match &selected {
				Selected::Single(..) => {}
				Selected::Multi(options) => {
					let options = options.iter().filter(|x| *x != &option).cloned().collect();
					(upper_on_select)(Selected::Multi(options));
				}
			}));

		let options = self.options.iter().map(|x| {
			SelectOptionComponent {
				option: x.clone(),
				on_select: on_select.clone(),
				on_deselect: on_deselect.clone(),
				is_selected: self.selected.is_selected(&x.id),
			}
			.into_element()
		});

		rect()
			.width(Size::fill())
			.cont()
			.main_align(if self.align_end {
				Alignment::End
			} else {
				Alignment::Start
			})
			.children(options)
	}
}

#[derive(PartialEq)]
struct SelectOptionComponent {
	option: SelectOption,
	on_select: Captured<Rc<dyn Fn(String)>>,
	on_deselect: Captured<Rc<dyn Fn(String)>>,
	is_selected: bool,
}

impl Component for SelectOptionComponent {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let id = self.option.id.clone();
		let on_select = self.on_select.clone();
		let on_deselect = self.on_select.clone();
		let is_selected = self.is_selected;
		let mut out = rect()
			.cont()
			.center()
			.corner_radius(theme.round)
			.height(Size::px(theme.input_height))
			.padding((6.0, 12.0))
			.item_colorway(&theme, *is_hovered.read(), self.is_selected)
			.on_press(move |_| {
				if is_selected {
					on_deselect(id.clone());
				} else {
					on_select(id.clone());
				}
			})
			.clickable();

		if let Some(ico) = &self.option.icon {
			out = out.child(icon(ico, 16.0));
		}

		out.child(self.option.title.as_str())
	}
}

#[derive(PartialEq, Clone)]
pub struct SelectOption {
	pub id: String,
	pub title: String,
	pub icon: Option<String>,
}

/// What's actually selected for a select component, supporting both single and multi select
#[derive(PartialEq, Clone)]
pub enum Selected {
	Single(String),
	Multi(Vec<String>),
}

impl Selected {
	/// Gets a single result out, panicking if it is none
	pub fn single(self) -> String {
		self.single_optional().unwrap()
	}

	/// Gets a single result out
	pub fn single_optional(self) -> Option<String> {
		match self {
			Self::Single(value) => Some(value),
			Self::Multi(values) => values.first().cloned(),
		}
	}

	/// Gets multiple results out
	pub fn multi(self) -> Vec<String> {
		match self {
			Self::Single(value) => vec![value],
			Self::Multi(values) => values,
		}
	}

	/// Checks whether this option is selected
	fn is_selected(&self, option: &str) -> bool {
		match self {
			Self::Single(value) => value == option,
			Self::Multi(values) => values.iter().any(|x| x == option),
		}
	}
}
