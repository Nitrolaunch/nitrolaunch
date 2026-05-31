use crate::prelude::*;

#[derive(PartialEq)]
pub struct TextInput {
	value: Writable<String>,
	on_change: Option<EventHandler<String>>,
	on_submit: Option<EventHandler<String>>,
}

impl TextInput {
	pub fn new(value: impl Into<Writable<String>>) -> Self {
		Self {
			value: value.into(),
			on_change: None,
			on_submit: None,
		}
	}

	pub fn on_change(mut self, on_change: impl Into<EventHandler<String>>) -> Self {
		self.on_change = Some(on_change.into());
		self
	}

	pub fn on_submit(mut self, on_submit: impl Into<EventHandler<String>>) -> Self {
		self.on_submit = Some(on_submit.into());
		self
	}
}

impl Component for TextInput {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		// rect()
		// 	.background(theme.bg)
		// 	.border(theme.border2(theme.item_border))
		// 	.corner_radius(theme.round2)
		let on_change = self.on_change.clone();
		let input_theme = InputColorsThemePartial {
			background: Some(Preference::Specific(theme.bg.into())),
			hover_background: Some(Preference::Specific(theme.panel.into())),
			border_fill: Some(Preference::Specific(theme.item_border.into())),
			focus_border_fill: Some(Preference::Specific(theme.item_select_border.into())),
			color: Some(Preference::Specific(theme.fg.into())),
			placeholder_color: Some(Preference::Specific(theme.fg3.into())),
		};

		Input::new(self.value.clone())
			.width(Size::fill())
			.theme_colors(input_theme)
			.corner_radius(theme.round2)
			.maybe(self.on_submit.is_some(), |this| {
				this.on_submit(self.on_submit.clone().unwrap())
			})
			.maybe(self.on_change.is_some(), |this| {
				this.on_validate(move |validator: InputValidator| {
					on_change.as_ref().unwrap().call(validator.text().clone());
				})
			})
	}
}
