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
}

impl<T: PartialEq + Clone + 'static> Component for SideTabs<T> {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let gap = 5.0;

		let tabs = self.tabs.iter().map(|tab| {
			let is_selected = (&*self.selected.read()) == &tab.id;
			let (bg, fg) = if is_selected {
				(theme.highlight.into(), theme.fg)
			} else {
				(Color::TRANSPARENT, theme.disabled)
			};

			let mut selected = self.selected.clone();
			let id = tab.id.clone();

			rect()
				.width(Size::fill())
				.height(Size::px(32.0))
				.background(bg)
				.color(fg)
				.corner_radius(theme.round)
				.margin(Gaps::new(0.0, 0.0, gap, 0.0))
				.horizontal()
				.spacing(8.0)
				.padding(12.0)
				.cross_align(Alignment::Center)
				.clickable()
				.on_press(move |_| selected.set(id.clone()))
				.maybe_child(tab.icon.clone())
				.child(tab.title.as_str())
				.into_element()
		});

		let out = rect().width(Size::fill()).margin(gap).children(tabs);

		ScrollView::new()
			.width(Size::fill())
			.height(Size::fill())
			.direction(Direction::Vertical)
			.child(out)
	}
}
