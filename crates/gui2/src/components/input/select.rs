use std::rc::Rc;

use crate::{components::input::Derivable, prelude::*};

#[derive(PartialEq)]
pub struct InlineSelect {
	options: Vec<SelectOption>,
	selected: Selected,
	on_select: NotEq<Rc<dyn Fn(Selected)>>,
	derived_option: Option<String>,
	align_end: bool,
	fit: bool,
}

impl InlineSelect {
	pub fn new(selected: Selected, on_select: Rc<dyn Fn(Selected)>) -> Self {
		Self {
			options: Vec::new(),
			selected,
			on_select: NotEq(on_select),
			derived_option: None,
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

impl Derivable<String> for InlineSelect {
	fn derived(mut self, value: Option<String>) -> Self {
		self.derived_option = value;
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
			let is_selected = self.selected.is_selected(&x.id);
			let is_derived =
				!is_selected && self.derived_option.as_ref().is_some_and(|y| y == &x.id);

			InlineSelectOption {
				option: x.clone(),
				on_select: on_select.clone(),
				on_deselect: on_deselect.clone(),
				is_selected,
				is_derived,
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
	is_derived: bool,
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
			.maybe(!self.is_derived, |this| {
				this.item_colorway(&theme, *is_hovered.read(), self.is_selected)
			})
			.maybe(self.is_derived, |this| {
				this.border(theme.border(theme.template))
					.color(theme.template)
					.background(theme.template_bg)
			})
			.maybe(self.is_selected, |this| this.font_weight(FontWeight::BOLD))
			.maybe(!self.fit, |this| this.width(Size::flex(1.0)))
			.on_press(move |_| {
				if is_selected {
					on_deselect.0(id.clone());
				} else {
					on_select.0(id.clone());
				}
			})
			.clickable()
			.maybe_child(self.option.icon.clone())
			.child(self.option.title.as_str())
	}
}

#[derive(PartialEq)]
pub struct Dropdown {
	selected: Selected,
	on_select: NotEq<Rc<dyn Fn(Selected)>>,
	options: Vec<SelectOption>,
	derived_option: Option<String>,
}

impl Dropdown {
	pub fn new(selected: Selected, on_select: Rc<dyn Fn(Selected)>) -> Self {
		Self {
			selected,
			on_select: NotEq(on_select),
			options: Vec::new(),
			derived_option: None,
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

impl Derivable<String> for Dropdown {
	fn derived(mut self, value: Option<String>) -> Self {
		self.derived_option = value;
		self
	}
}

impl Component for Dropdown {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let mut is_open = use_state(|| false);

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

		let option_count = if self.options.len() > 5 {
			5.5
		} else {
			self.options.len() as f32
		};
		let options_height = (theme.input_height + theme.gap) * option_count;

		let selected = self.selected.clone();
		let derived_option = self.derived_option.clone();
		let options = self.options.clone();
		let len = self.options.len();
		let options = VirtualScrollView::new(move |i, _| {
			// Extra element to fix cutoff of the last element in the scroll
			if i == len {
				return rect().into_element();
			}
			let option = options.get(i).unwrap();

			let is_selected = selected.is_selected(&option.id);
			let is_derived =
				!is_selected && derived_option.as_ref().is_some_and(|y| y == &option.id);

			DropdownOption {
				option: option.clone(),
				on_select: on_select.clone(),
				on_deselect: on_deselect.clone(),
				is_selected,
				is_derived,
			}
			.into_element()
		})
		.length(self.options.len() + 1)
		.item_size(theme.input_height)
		.width(Size::fill())
		.height(Size::fill());

		let options = rect()
			.width(Size::fill())
			.height(Size::px(options_height))
			.position(Position::new_absolute().top(theme.input_height + 8.0))
			.layer(Layer::Overlay)
			.panel_colorway(&theme, false, false)
			.corner_radius(theme.round)
			.padding(theme.gap)
			.on_pointer_leave(move |_| {
				is_open.set(false);
			})
			.child(options);

		header.maybe(*is_open.read(), |this| this.child(options))
	}
}

#[derive(PartialEq)]
struct DropdownOption {
	option: SelectOption,
	on_select: NotEq<Rc<dyn Fn(String)>>,
	on_deselect: NotEq<Rc<dyn Fn(String)>>,
	is_selected: bool,
	is_derived: bool,
}

impl Component for DropdownOption {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let on_select = self.on_select.clone();
		let on_deselect = self.on_deselect.clone();
		let id = self.option.id.clone();
		let is_selected = self.is_selected;

		let (fg, bg, border) = if self.is_derived {
			(theme.template, theme.template_bg, theme.template)
		} else if self.is_selected {
			(theme.item_select_border, theme.item_select, theme.item_select_border)
		} else if *is_hovered.read() {
			(theme.fg, theme.highlight, theme.panel)
		} else {
			(theme.fg, theme.panel, theme.panel)
		};

		rect()
			.width(Size::fill())
			.height(Size::px(theme.input_height))
			.color(fg)
			.background(bg)
			.border(theme.border(border))
			.corner_radius(theme.round)
			.margin(Gaps::new(0.0, 0.0, theme.gap, 0.0))
			.maybe(self.is_selected, |this| this.font_weight(FontWeight::BOLD))
			.cont()
			.center()
			.clickable()
			.hover(is_hovered)
			.on_press(move |_| {
				if is_selected {
					(on_deselect.0)(id.clone());
				} else {
					(on_select.0)(id.clone());
				}
			})
			.maybe_child(self.option.icon.clone())
			.child(self.option.title.as_str())
			.into_element()
	}
}

#[derive(PartialEq, Clone)]
pub struct SelectOption {
	pub id: String,
	pub title: String,
	pub icon: Option<Element>,
}

impl SelectOption {
	pub fn simple(id: &str) -> Self {
		Self::new(id, id, None)
	}

	pub fn new(id: &str, title: &str, ico: Option<&str>) -> Self {
		Self {
			id: id.into(),
			title: title.into(),
			icon: ico.map(|x| icon(x, 16.0).into_element()),
		}
	}

	pub fn new_custom_icon(id: &str, title: &str, ico: Element) -> Self {
		Self {
			id: id.into(),
			title: title.into(),
			icon: Some(ico),
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
