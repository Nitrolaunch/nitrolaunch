use std::rc::Rc;

use freya::query::Captured;

use crate::prelude::*;

#[derive(PartialEq)]
pub struct InlineSelect {
	options: Vec<SelectOption>,
	selected: Option<String>,
	on_select: Captured<Rc<dyn Fn(String)>>,
	align_end: bool,
}

impl InlineSelect {
	pub fn new(selected: Option<String>, on_select: Rc<dyn Fn(String)>) -> Self {
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

	pub fn align_end(mut self) -> Self {
		self.align_end = true;
		self
	}
}

impl Component for InlineSelect {
	fn render(&self) -> impl IntoElement {
		let options = self.options.iter().map(|x| {
			SelectOptionComponent {
				option: x.clone(),
				on_select: self.on_select.clone(),
				is_selected: self.selected.as_ref().is_some_and(|y| y == &x.id),
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
	is_selected: bool,
}

impl Component for SelectOptionComponent {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let id = self.option.id.clone();
		let on_select = self.on_select.clone();
		let mut out = rect()
			.cont()
			.center()
			.corner_radius(theme.round)
			.height(Size::px(theme.input_height))
			.padding(6.0)
			.item_colorway(&theme, *is_hovered.read(), self.is_selected)
			.on_press(move |_| on_select(id.clone()))
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
