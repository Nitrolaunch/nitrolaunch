use crate::prelude::*;

#[derive(PartialEq)]
pub struct SideTabs {
	tabs: Vec<SelectOption>,
	selected: State<Option<String>>,
}

impl SideTabs {
	pub fn new(selected: State<Option<String>>) -> Self {
		Self {
			tabs: Vec::new(),
			selected,
		}
	}

	pub fn child(mut self, child: SelectOption) -> Self {
		self.tabs.push(child);
		self
	}
}

impl Component for SideTabs {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let gap = 5.0;

		let tabs = self.tabs.iter().map(|tab| {
			let is_selected = self.selected.read().as_ref().is_some_and(|x| *x == tab.id);
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
				.on_press(move |_| selected.set(Some(id.clone())))
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
