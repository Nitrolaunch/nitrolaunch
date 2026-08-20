use nitrolaunch::config_crate::template::TemplateConfig;

use crate::{prelude::*, state::FrontState, util::Shared};

pub mod control;
pub mod file;
pub mod icon;
pub mod select;
pub mod switch;
pub mod tabs;
pub mod text;

/// A label above a configuration field
pub fn field_label(text: &str, ico: &str, theme: &Theme) -> impl IntoElement {
	rect()
		.width(Size::fill())
		.horizontal()
		.spacing(theme.gap)
		.main_align(Alignment::Start)
		.cross_align(Alignment::Center)
		.font_size(13)
		.font_weight(FontWeight::BOLD)
		.color(theme.fg3)
		.padding(Gaps::new(0.0, 0.0, theme.gap, theme.gap))
		.child(icon(ico, 12.0))
		.child(text)
}

/// Configuration field with a label
pub fn field(label: &str, icon: &str, theme: &Theme, field: impl IntoElement) -> Rect {
	rect()
		.width(Size::fill())
		.margin(Gaps::new(0.0, 0.0, 18.0, 0.0))
		.child(field_label(label, icon, theme))
		.child(field)
}

/// Used for inputs that can display derived values
pub trait Derivable<T>: Sized {
	fn derived(self, value: Option<T>) -> Self;

	fn derived_value<'a>(
		self,
		editable_value: Option<&'a T>,
		parent_configs: &'a [TemplateConfig],
		property: impl Fn(&'a TemplateConfig) -> Option<&'a T>,
	) -> Self
	where
		T: Clone,
	{
		self.derived(derived_value(editable_value, parent_configs, property).cloned())
	}

	fn derived_value_owned(
		self,
		editable_value: Option<T>,
		parent_configs: &[TemplateConfig],
		property: impl Fn(&TemplateConfig) -> Option<T>,
	) -> Self {
		self.derived(derived_value_owned(
			editable_value,
			parent_configs,
			property,
		))
	}
}

pub fn derived_value<'a, T>(
	editable_value: Option<&'a T>,
	parent_configs: &'a [TemplateConfig],
	property: impl Fn(&'a TemplateConfig) -> Option<&'a T>,
) -> Option<&'a T> {
	if editable_value.is_some() {
		None
	} else {
		parent_configs.iter().find_map(property)
	}
}

pub fn derived_value_owned<T>(
	editable_value: Option<T>,
	parent_configs: &[TemplateConfig],
	property: impl Fn(&TemplateConfig) -> Option<T>,
) -> Option<T> {
	if editable_value.is_some() {
		None
	} else {
		parent_configs.iter().find_map(property)
	}
}

pub fn final_value_owned<T>(
	editable_value: Option<T>,
	parent_configs: &[TemplateConfig],
	property: impl Fn(&TemplateConfig) -> Option<T>,
) -> Option<T> {
	if let Some(value) = editable_value {
		Some(value)
	} else {
		parent_configs.iter().find_map(property)
	}
}

/// Used for inputs that can display validation errors
pub trait InputError {
	fn input_error(self, message: &str) -> Self;

	fn maybe_input_error(self, condition: bool, message: &str) -> Self
	where
		Self: Sized,
	{
		if condition {
			self.input_error(message)
		} else {
			self
		}
	}
}

/// Simple absolute error message tag
pub fn input_error(message: &str, theme: &Theme) -> impl IntoElement {
	rect()
		.position(Position::new_absolute().top(-theme.gap2).right(-theme.gap2))
		.layer(Layer::Relative(2))
		.padding((theme.gap / 2.0, theme.gap))
		.corner_radius(theme.round)
		.background(theme.error)
		.font_size(theme.font0)
		.child(message)
}

pub fn slider(
	value: f64,
	min: f64,
	max: f64,
	step: f64,
	on_change: impl Into<EventHandler<f64>>,
	theme: &Theme,
	front_state: &Shared<FrontState>,
) -> impl IntoElement {
	let on_change = on_change.into();
	let scale = (max - min) / 100.0;
	let scaled_value = value / scale;

	let theme = SliderThemePartial {
		background: Some(Preference::Specific(theme.panel)),
		border_fill: Some(Preference::Specific(theme.panel_border)),
		thumb_background: Some(Preference::Specific(theme.primary)),
		thumb_inner_background: Some(Preference::Specific(theme.primary)),
	};

	let slider = Slider::new(move |new_value: f64| {
		let new_value = new_value * scale + min;
		let rounded = (new_value / step).round() * step;
		on_change.call(rounded);
	})
	.value(scaled_value)
	.theme(theme);

	rect()
		.width(Size::px(240.0))
		.tip(&front_state, &format!("{:.3}", value))
		.child(slider)
		.into_element()
}
