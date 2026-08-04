use crate::{
	components::input::{Derivable, input_error},
	prelude::*,
};

#[derive(PartialEq)]
pub struct TextInput {
	value: Writable<String>,
	on_change: Option<EventHandler<String>>,
	on_submit: Option<EventHandler<String>>,
	derived_value: Option<String>,
	placeholder: Option<String>,
	error: Option<String>,
}

#[allow(dead_code)]
impl TextInput {
	pub fn new(value: impl Into<Writable<String>>) -> Self {
		Self {
			value: value.into(),
			on_change: None,
			on_submit: None,
			derived_value: None,
			placeholder: None,
			error: None,
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

	pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
		self.placeholder = Some(placeholder.into());
		self
	}
}

impl Derivable<String> for TextInput {
	fn derived(mut self, value: Option<String>) -> Self {
		self.derived_value = value;
		self
	}
}

impl InputError for TextInput {
	fn input_error(mut self, message: &str) -> Self {
		self.error = Some(message.into());
		self
	}
}

impl Component for TextInput {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let on_change = self.on_change.clone();
		let input_theme = InputColorsThemePartial {
			background: Some(Preference::Specific(theme.bg.into())),
			focus_background: Some(Preference::Specific(theme.panel.into())),
			border_fill: Some(Preference::Specific(theme.item_border.into())),
			focus_border_fill: Some(Preference::Specific(theme.item_select_border.into())),
			color: Some(Preference::Specific(theme.fg.into())),
			placeholder_color: Some(Preference::Specific(theme.fg3.into())),
		};

		let out = Input::new(self.value.clone())
			.width(Size::fill())
			.theme_colors(input_theme)
			.corner_radius(theme.round)
			.maybe(self.derived_value.is_some(), |this| {
				this.placeholder(self.derived_value.clone().unwrap())
					.placeholder_color(theme.template)
			})
			.maybe(self.on_submit.is_some(), |this| {
				this.on_submit(self.on_submit.clone().unwrap())
			})
			.maybe(self.on_change.is_some(), |this| {
				this.on_validate(move |validator: InputValidator| {
					on_change.as_ref().unwrap().call(validator.text().clone());
				})
			})
			.maybe(self.placeholder.is_some(), |this| {
				this.placeholder(self.placeholder.clone().unwrap())
			})
			.maybe(self.error.is_some(), |this| {
				this.border_fill(theme.error).focus_border_fill(theme.error)
			})
			.into_element();

		rect()
			.width(Size::fill())
			.child(out)
			.maybe(self.error.is_some(), |this| {
				this.child(input_error(self.error.as_ref().unwrap(), &theme))
			})
	}
}

pub fn search_bar(input: TextInput, theme: &Theme) -> Rect {
	rect()
		.width(Size::fill())
		.child(
			rect()
				.position(Position::new_absolute().right(12.0))
				.height(Size::px(theme.input_height))
				.center()
				.child(icon("search", 16.0)),
		)
		.child(input)
}

pub fn transparent_text_input(input: State<String>, theme: &Theme) -> Input {
	let input_theme = InputColorsThemePartial {
		background: Some(Preference::Specific(Color::TRANSPARENT)),
		focus_background: Some(Preference::Specific(Color::TRANSPARENT)),
		border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
		focus_border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
		color: Some(Preference::Specific(theme.fg.into())),
		placeholder_color: Some(Preference::Specific(theme.fg3.into())),
	};

	Input::new(input)
		.width(Size::fill())
		.theme_colors(input_theme)
}
