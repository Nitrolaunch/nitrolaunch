use std::rc::Rc;

use crate::{components::input::Derivable, prelude::*, theme::Colorway};

#[derive(PartialEq)]
pub struct InlineSelect<T: PartialEq + Clone> {
	options: Vec<SelectOption<T>>,
	selected: Selected<T>,
	on_select: NotEq<Rc<dyn Fn(Selected<T>)>>,
	derived_option: Option<T>,
	align_end: bool,
	fit: bool,
}

#[allow(dead_code)]
impl<T: PartialEq + Clone> InlineSelect<T> {
	pub fn new(selected: Selected<T>, on_select: Rc<dyn Fn(Selected<T>)>) -> Self {
		Self {
			options: Vec::new(),
			selected,
			on_select: NotEq(on_select),
			derived_option: None,
			align_end: false,
			fit: false,
		}
	}

	pub fn child(mut self, child: SelectOption<T>) -> Self {
		self.options.push(child);
		self
	}

	pub fn maybe_child(mut self, show: bool, child: impl FnOnce() -> SelectOption<T>) -> Self {
		if show {
			self.options.push(child());
		}
		self
	}

	pub fn children(mut self, children: impl IntoIterator<Item = SelectOption<T>>) -> Self {
		for child in children {
			self = self.child(child);
		}
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

impl<T: PartialEq + Clone> InlineSelect<Option<T>> {
	pub fn allow_none(mut self) -> Self {
		self.options.push(SelectOption::none());
		self
	}
}

impl<T: PartialEq + Clone> Derivable<T> for InlineSelect<T> {
	fn derived(mut self, value: Option<T>) -> Self {
		self.derived_option = value;
		self
	}
}

impl<T: PartialEq + Clone + 'static> Component for InlineSelect<T> {
	fn render(&self) -> impl IntoElement {
		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_select: NotEq<Rc<dyn Fn(T)>> = NotEq(Rc::new(move |option| {
			(upper_on_select.0)(selected.clone().select(&option));
		}));

		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_deselect: NotEq<Rc<dyn Fn(T)>> = NotEq(Rc::new(move |option| {
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
struct InlineSelectOption<T: PartialEq + Clone> {
	option: SelectOption<T>,
	on_select: NotEq<Rc<dyn Fn(T)>>,
	on_deselect: NotEq<Rc<dyn Fn(T)>>,
	is_selected: bool,
	is_derived: bool,
	fit: bool,
}

impl<T: PartialEq + Clone + 'static> Component for InlineSelectOption<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let front_state = use_front_state();

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
			.maybe(
				self.is_selected && self.option.selected_colorway.is_some(),
				|mut this| {
					this.get_style().borders.clear();
					this.colorway(self.option.selected_colorway.unwrap(), &theme)
				},
			)
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
			.maybe(self.option.tip.is_some(), |this| {
				this.tip(&front_state, self.option.tip.as_deref().unwrap())
			})
			.maybe_child(Some(self.option.title.as_str()).filter(|x| !x.is_empty()))
	}
}

#[derive(PartialEq)]
pub struct Dropdown<T: PartialEq + Clone> {
	selected: Selected<T>,
	on_select: NotEq<Rc<dyn Fn(Selected<T>)>>,
	options: Vec<SelectOption<T>>,
	derived_option: Option<T>,
}

#[allow(dead_code)]
impl<T: PartialEq + Clone> Dropdown<T> {
	pub fn new(selected: Selected<T>, on_select: Rc<dyn Fn(Selected<T>)>) -> Self {
		Self {
			selected,
			on_select: NotEq(on_select),
			options: Vec::new(),
			derived_option: None,
		}
	}

	pub fn child(mut self, child: SelectOption<T>) -> Self {
		self.options.push(child);
		self
	}

	pub fn maybe_child(mut self, show: bool, child: impl FnOnce() -> SelectOption<T>) -> Self {
		if show {
			self.options.push(child());
		}
		self
	}

	pub fn children(mut self, children: impl IntoIterator<Item = SelectOption<T>>) -> Self {
		for child in children {
			self = self.child(child);
		}
		self
	}
}

impl<T: PartialEq + Clone> Dropdown<Option<T>> {
	pub fn allow_none(mut self) -> Self {
		self.options.push(SelectOption::none());
		self
	}
}

impl<T: PartialEq + Clone> Derivable<T> for Dropdown<T> {
	fn derived(mut self, value: Option<T>) -> Self {
		self.derived_option = value;
		self
	}
}

impl<T: PartialEq + Clone + 'static> Component for Dropdown<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let mut is_open = use_state(|| false);

		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_select: NotEq<Rc<dyn Fn(T)>> = NotEq(Rc::new(move |option| {
			(upper_on_select.0)(selected.clone().select(&option));
		}));

		let selected = self.selected.clone();
		let upper_on_select = self.on_select.clone();
		let on_deselect: NotEq<Rc<dyn Fn(T)>> = NotEq(Rc::new(move |option| {
			(upper_on_select.0)(selected.clone().deselect(&option));
		}));

		let preview = match &self.selected {
			Selected::Single(selected) => {
				if let Some(option) = self.options.iter().find(|x| x.id == *selected) {
					option.title.clone()
				} else {
					"Unknown option".to_string()
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

		let option_count = if self.options.len() > 7 {
			7.5
		} else {
			self.options.len() as f32
		};
		let options_height = (theme.input_height + theme.gap) * option_count;

		let selected = self.selected.clone();
		let derived_option = self.derived_option.clone();
		let options = self.options.clone();
		let options = VirtualScrollView::new(move |i, _| {
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
		.length(self.options.len())
		.item_size(theme.input_height + theme.gap)
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
struct DropdownOption<T: PartialEq + Clone> {
	option: SelectOption<T>,
	on_select: NotEq<Rc<dyn Fn(T)>>,
	on_deselect: NotEq<Rc<dyn Fn(T)>>,
	is_selected: bool,
	is_derived: bool,
}

impl<T: PartialEq + Clone + 'static> Component for DropdownOption<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);
		let front_state = use_front_state();

		let on_select = self.on_select.clone();
		let on_deselect = self.on_deselect.clone();
		let id = self.option.id.clone();
		let is_selected = self.is_selected;

		let (fg, bg, border) = if self.is_derived {
			(theme.template, theme.template_bg, theme.template)
		} else if self.is_selected {
			(
				theme.item_select_border,
				theme.item_select,
				theme.item_select_border,
			)
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
			.maybe(self.option.tip.is_some(), |this| {
				this.tip(&front_state, self.option.tip.as_deref().unwrap())
			})
			.child(self.option.title.as_str())
			.into_element()
	}
}

#[derive(PartialEq, Clone)]
pub struct SelectOption<T: PartialEq + Clone> {
	pub id: T,
	pub title: String,
	pub icon: Option<Element>,
	pub tip: Option<String>,
	pub selected_colorway: Option<Colorway>,
}

impl<T: PartialEq + Clone> SelectOption<T> {
	pub fn simple(id: impl Into<T>) -> Self
	where
		T: ToString,
	{
		let id = id.into();
		let title = id.to_string();
		Self::new(id, &title, None)
	}

	pub fn new(id: impl Into<T>, title: &str, ico: Option<&str>) -> Self {
		Self {
			id: id.into(),
			title: title.into(),
			icon: ico.map(|x| icon(x, 16.0).into_element()),
			tip: None,
			selected_colorway: None,
		}
	}

	pub fn new_custom_icon(id: impl Into<T>, title: &str, ico: Element) -> Self {
		Self {
			id: id.into(),
			title: title.into(),
			icon: Some(ico),
			tip: None,
			selected_colorway: None,
		}
	}

	pub fn tip(mut self, tip: &str) -> Self {
		self.tip = Some(tip.into());
		self
	}

	pub fn selected_colorway(mut self, colorway: Colorway) -> Self {
		self.selected_colorway = Some(colorway);
		self
	}
}

impl<T: Clone + PartialEq> SelectOption<Option<T>> {
	pub fn simple_or_none(id: Option<T>) -> Self
	where
		T: ToString,
	{
		if let Some(id) = id {
			let title = id.to_string();
			Self::new(id, &title, None)
		} else {
			Self::none()
		}
	}

	pub fn none() -> Self {
		Self::new(None, "None", None)
	}
}

/// What's actually selected for a select component, supporting both single and multi select
#[derive(PartialEq, Clone)]
pub enum Selected<T: PartialEq + Clone> {
	Single(T),
	Multi(Vec<T>),
}

impl<T: PartialEq + Clone> Selected<T> {
	/// Gets a single result out, panicking if it is none
	pub fn single(self) -> T {
		match self {
			Self::Single(value) => value,
			_ => unreachable!(),
		}
	}

	/// Gets a single result out
	pub fn single_optional(self) -> Option<T> {
		match self {
			Self::Single(value) => Some(value),
			Self::Multi(values) => values.first().cloned(),
		}
	}

	/// Gets multiple results out
	pub fn multi(self) -> Vec<T> {
		match self {
			Self::Single(value) => vec![value],
			Self::Multi(values) => values,
		}
	}

	/// Checks whether this option is selected
	fn is_selected(&self, option: &T) -> bool {
		match self {
			Self::Single(value) => value == option,
			Self::Multi(values) => values.iter().any(|x| x == option),
		}
	}

	fn select(self, new: &T) -> Self {
		match self {
			Self::Single(..) => Self::Single(new.clone()),
			Self::Multi(mut list) => {
				list.push(new.clone());
				Self::Multi(list)
			}
		}
	}

	fn deselect(self, value: &T) -> Self {
		match self {
			Self::Single(..) => self,
			Self::Multi(list) => {
				let list = list.into_iter().filter(|x| x != value).collect();
				Self::Multi(list)
			}
		}
	}
}
