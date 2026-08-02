use crate::prelude::*;

#[derive(PartialEq)]
pub struct SideTabs<T: PartialEq + Clone + 'static> {
	tabs: Vec<SelectOption<T>>,
	selected: State<T>,
}

impl<T: PartialEq + Clone + 'static> SideTabs<T> {
	pub fn new(selected: State<T>) -> Self {
		Self {
			tabs: Vec::new(),
			selected,
		}
	}

	pub fn child(mut self, child: SelectOption<T>) -> Self {
		self.tabs.push(child);
		self
	}

	pub fn children(mut self, children: impl Iterator<Item = SelectOption<T>>) -> Self {
		self.tabs.extend(children);
		self
	}
}

impl<T: PartialEq + Clone + 'static> Component for SideTabs<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let tabs = self.tabs.iter().map(|tab| {
			Tab {
				is_selected: (&*self.selected.read()) == &tab.id,
				option: tab.clone(),
				on_select: self.selected.setter(),
				horizontal: false,
			}
			.into_element()
		});

		let out = rect()
			.width(Size::fill())
			.margin(theme.gap)
			.spacing(theme.gap)
			.children(tabs);

		ScrollView::new()
			.width(Size::fill())
			.height(Size::fill())
			.direction(Direction::Vertical)
			.child(out)
	}
}

#[derive(PartialEq)]
pub struct TopTabs<T: PartialEq + Clone + 'static> {
	tabs: Vec<SelectOption<T>>,
	selected: State<T>,
}

impl<T: PartialEq + Clone + 'static> TopTabs<T> {
	pub fn new(selected: State<T>) -> Self {
		Self {
			tabs: Vec::new(),
			selected,
		}
	}

	pub fn child(mut self, child: SelectOption<T>) -> Self {
		self.tabs.push(child);
		self
	}

	pub fn children(mut self, children: impl Iterator<Item = SelectOption<T>>) -> Self {
		self.tabs.extend(children);
		self
	}
}

impl<T: PartialEq + Clone + 'static> Component for TopTabs<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let tabs = self.tabs.iter().map(|tab| {
			Tab {
				is_selected: (&*self.selected.read()) == &tab.id,
				option: tab.clone(),
				on_select: self.selected.setter(),
				horizontal: true,
			}
			.into_element()
		});

		rect()
			.width(Size::fill())
			.cont()
			.margin(theme.gap)
			.children(tabs)
	}
}

#[derive(PartialEq)]
pub struct Tab<T: PartialEq + Clone> {
	pub option: SelectOption<T>,
	pub is_selected: bool,
	pub on_select: EventHandler<T>,
	pub horizontal: bool,
}

impl<T: PartialEq + Clone + 'static> Component for Tab<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let (bg, fg) = if self.is_selected {
			(theme.highlight, theme.fg)
		} else if *is_hovered.read() {
			(theme.panel_hover, theme.fg3)
		} else {
			(Color::TRANSPARENT, theme.disabled)
		};

		let on_select = self.on_select.clone();
		let id = self.option.id.clone();

		rect()
			.width(Size::flex(1.0))
			.height(Size::px(32.0))
			.background(bg)
			.color(fg)
			.corner_radius(theme.round)
			.horizontal()
			.spacing(8.0)
			.padding(12.0)
			.cross_align(Alignment::Center)
			.maybe(self.horizontal, |this| this.main_align(Alignment::Center))
			.hover(is_hovered)
			.on_press(move |_| on_select.call(id.clone()))
			.maybe_child(self.option.icon.clone())
			.child(label().text(self.option.title.to_string()).max_lines(1))
			.into_element()
	}
}
