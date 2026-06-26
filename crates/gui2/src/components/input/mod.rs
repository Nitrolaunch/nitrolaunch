use nitrolaunch::config_crate::template::TemplateConfig;

use crate::prelude::*;

pub mod control;
pub mod icon;
pub mod select;
pub mod switch;
pub mod tabs;
pub mod text;

/// A label above a configuration field
pub fn field_label(text: &str, theme: &Theme) -> impl IntoElement {
	rect()
		.width(Size::fill())
		.horizontal()
		.main_align(Alignment::Start)
		.cross_align(Alignment::End)
		.font_size(13)
		.font_weight(FontWeight::BOLD)
		.color(theme.fg3)
		.padding(Gaps::new(0.0, 0.0, 5.0, 3.0))
		.child(text)
}

/// Configuration field with a label
pub fn field(label: &str, theme: &Theme, field: impl IntoElement) -> Rect {
	rect()
		.width(Size::fill())
		.margin(Gaps::new(0.0, 0.0, 12.0, 0.0))
		.child(field_label(label, theme))
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
		self.derived(derived_value_owned(editable_value, parent_configs, property))
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
		parent_configs.into_iter().find_map(|x| property(x))
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
		parent_configs.into_iter().find_map(|x| property(x))
	}
}
