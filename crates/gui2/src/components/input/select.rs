use std::rc::Rc;

use crate::prelude::*;

#[derive(PartialEq)]
pub struct InlineSelect {
	options: Vec<SelectOption>,
	selected: Selected,
	on_select: NotEq<Rc<dyn Fn(Selected)>>,
	align_end: bool,
	fit: bool,
}

impl InlineSelect {
	pub fn new(selected: Selected, on_select: Rc<dyn Fn(Selected)>) -> Self {
		Self {
			options: Vec::new(),
			selected,
			on_select: NotEq(on_select),
			align_end: false,
			fit: false,
		}
	}

	pub fn child(mut self, child: SelectOption) -> Self {
		self.options.push(child);
		self
	}

	pub fn maybe_child(mut self, show: bool, child: impl FnOnce() -> SelectOption) -> Self {
		if show {
			self.options.push(child());
		}
		self
	}

	pub fn children(mut self, children: impl IntoIterator<Item = SelectOption>) -> Self {
		for child in children {
			self = self.child(child);
		}
		self
	}

	pub fn allow_none(mut self) -> Self {
		self.options.push(SelectOption::none());
		self
	}

	pub fn align_end(mut self) -> Self {
		self.align_end = true;
		self
	}

	/// Make the options fit the content instead of expanding to the full width
	pub fn fit(mut self) -> Self {
		self.fit = true;
		self
	}
}

impl Component for InlineSelect {
	fn render(&self) -> impl IntoElement {
		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_select: NotEq<Rc<dyn Fn(String)>> = NotEq(Rc::new(move |option| {
			(upper_on_select.0)(selected.clone().select(&option));
		}));

		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_deselect: NotEq<Rc<dyn Fn(String)>> = NotEq(Rc::new(move |option| {
			(upper_on_select.0)(selected.clone().deselect(&option));
		}));

		let options = self.options.iter().map(|x| {
			InlineSelectOption {
				option: x.clone(),
				on_select: on_select.clone(),
				on_deselect: on_deselect.clone(),
				is_selected: self.selected.is_selected(&x.id),
				fit: self.fit,
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
struct InlineSelectOption {
	option: SelectOption,
	on_select: NotEq<Rc<dyn Fn(String)>>,
	on_deselect: NotEq<Rc<dyn Fn(String)>>,
	is_selected: bool,
	fit: bool,
}

impl Component for InlineSelectOption {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let id = self.option.id.clone();
		let on_select = self.on_select.clone();
		let on_deselect = self.on_deselect.clone();
		let is_selected = self.is_selected;

		rect()
			.cont()
			.center()
			.corner_radius(theme.round)
			.height(Size::px(theme.input_height))
			.padding((6.0, 12.0))
			.item_colorway(&theme, *is_hovered.read(), self.is_selected)
			.maybe(!self.fit, |this| this.width(Size::flex(1.0)))
			.on_press(move |_| {
				if is_selected {
					on_deselect.0(id.clone());
				} else {
					on_select.0(id.clone());
				}
			})
			.clickable()
			.maybe_child(self.option.icon.as_ref().map(|x| icon(x, 16.0)))
			.child(self.option.title.as_str())
	}
}

#[derive(PartialEq)]
pub struct Dropdown {
	selected: Selected,
	on_select: NotEq<Rc<dyn Fn(Selected)>>,
	options: Vec<SelectOption>,
}

impl Dropdown {
	pub fn new(selected: Selected, on_select: Rc<dyn Fn(Selected)>) -> Self {
		Self {
			selected,
			on_select: NotEq(on_select),
			options: Vec::new(),
		}
	}

	pub fn child(mut self, child: SelectOption) -> Self {
		self.options.push(child);
		self
	}

	pub fn maybe_child(mut self, show: bool, child: impl FnOnce() -> SelectOption) -> Self {
		if show {
			self.options.push(child());
		}
		self
	}

	pub fn children(mut self, children: impl IntoIterator<Item = SelectOption>) -> Self {
		for child in children {
			self = self.child(child);
		}
		self
	}

	pub fn allow_none(mut self) -> Self {
		self.options.push(SelectOption::none());
		self
	}
}

impl Component for Dropdown {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let mut is_open = use_state(|| false);

		let preview = match &self.selected {
			Selected::Single(selected) => {
				if let Some(option) = self.options.iter().find(|x| x.id == *selected) {
					option.title.clone()
				} else {
					selected.clone()
				}
			}
			Selected::Multi(selected) => format!("{} selected", selected.len()),
		};

		let header = rect()
			.width(Size::fill())
			.height(Size::px(theme.input_height))
			.corner_radius(theme.round)
			.item_colorway(&theme, *is_hovered.read(), false)
			.hover(is_hovered)
			.on_press(move |_| is_open.toggle())
			.center()
			.child(preview);

		let gap = 5.0;
		let option_count = if self.options.len() > 5 {
			5.5
		} else {
			self.options.len() as f32
		};
		let options_height = (theme.input_height + gap) * option_count;

		let theme2 = theme.clone();
		let options = self.options.clone();
		let selected = self.selected.clone();
		let on_select = self.on_select.clone();
		let options = VirtualScrollView::new(move |i, _| {
			let option = options.get(i).unwrap();
			let is_selected = selected.is_selected(&option.id);
			let on_select = on_select.clone();
			let selected = selected.clone();
			let id = option.id.clone();

			let (fg, bg, border) = if is_selected {
				(theme2.primary, theme2.primary_bg, theme2.primary)
			} else {
				(theme2.fg, theme2.panel, theme2.panel)
			};

			rect()
				.key(i)
				.width(Size::fill())
				.height(Size::px(theme2.input_height))
				.color(fg)
				.background(bg)
				.border(theme2.border(border))
				.corner_radius(theme2.round)
				.margin(Gaps::new(0.0, gap, gap, gap))
				.center()
				.clickable()
				.on_press(move |_| {
					if is_selected {
						(on_select.0)(selected.clone().deselect(&id));
					} else {
						(on_select.0)(selected.clone().select(&id));
					}
				})
				.child(option.title.as_str())
				.into_element()
		})
		.length(self.options.len())
		.item_size(theme.input_height)
		.width(Size::fill())
		.height(Size::px(options_height));

		let options = rect()
			.width(Size::fill())
			.position(Position::new_absolute().top(theme.input_height + 8.0))
			.layer(Layer::Overlay)
			.panel_colorway(&theme, false, false)
			.corner_radius(theme.round)
			.padding(gap)
			.on_pointer_leave(move |_| {
				is_open.set(false);
			})
			.child(options);

		header.maybe(*is_open.read(), |this| this.child(options))
	}
}

#[derive(PartialEq, Clone)]
pub struct SelectOption {
	pub id: String,
	pub title: String,
	pub icon: Option<String>,
}

impl SelectOption {
	pub fn simple(id: &str) -> Self {
		Self::new(id, id, None)
	}

	pub fn new(id: &str, title: &str, icon: Option<&str>) -> Self {
		Self {
			id: id.into(),
			title: title.into(),
			icon: icon.map(ToString::to_string),
		}
	}

	pub fn none() -> Self {
		Self::new("none", "None", None)
	}
}

/// What's actually selected for a select component, supporting both single and multi select
#[derive(PartialEq, Clone)]
pub enum Selected {
	Single(String),
	Multi(Vec<String>),
}

impl Selected {
	pub fn new_single(value: Option<String>) -> Self {
		let value = match value {
			Some(value) => value,
			None => "none".into(),
		};

		Self::Single(value)
	}

	/// Gets a single result out, panicking if it is none
	pub fn single(self) -> String {
		match self {
			Self::Single(value) => value,
			_ => unreachable!(),
		}
	}

	/// Gets a single result out
	pub fn single_optional(self) -> Option<String> {
		match self {
			Self::Single(value) if value == "none" => None,
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

	fn select(self, new: &str) -> Self {
		match self {
			Self::Single(..) => Self::Single(new.into()),
			Self::Multi(mut list) => {
				list.push(new.into());
				Self::Multi(list)
			}
		}
	}

	fn deselect(self, value: &str) -> Self {
		match self {
			Self::Single(..) => self,
			Self::Multi(list) => {
				let list = list.into_iter().filter(|x| x != value).collect();
				Self::Multi(list)
			}
		}
	}
}
