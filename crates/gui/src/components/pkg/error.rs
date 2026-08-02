use crate::{
	components::pkg::PackageChip, output::SerializableResolutionError, prelude::*, util::PtrEq,
};

use itertools::Itertools;

#[derive(PartialEq)]
pub struct ResolutionErrorView {
	pub error: PtrEq<SerializableResolutionError>,
}

impl Component for ResolutionErrorView {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_open = use_state(|| false);

		let ico = match &*self.error.0 {
			SerializableResolutionError::PackageContext(..) => "elipsis",
			SerializableResolutionError::FailedToPreload(..) => "error",
			SerializableResolutionError::FailedToGetProperties(..) => "curly_braces",
			SerializableResolutionError::NoValidVersionsFound(..) => "asterisk",
			SerializableResolutionError::ExtensionNotFulfilled(..) => "link_broken",
			SerializableResolutionError::ExplicitRequireNotFulfilled(..) => "link_broken",
			SerializableResolutionError::IncompatiblePackage(..) => "delete",
			SerializableResolutionError::FailedToEvaluate(..) => "error",
			SerializableResolutionError::Misc(..) => "error",
		};
		let ico = rect()
			.width(Size::px(48.0))
			.height(Size::px(48.0))
			.center()
			.child(icon(ico, 16.0));

		let title = get_res_err_title(&self.error.0);
		let contents = get_res_err_contents(&self.error.0);

		let arrow = if *is_open.read() {
			"angle_down"
		} else {
			"angle_right"
		};
		let arrow = rect()
			.width(Size::px(48.0))
			.height(Size::px(48.0))
			.center()
			.child(icon(arrow, 16.0));

		let mut is_open2 = is_open.clone();
		let header = rect()
			.width(Size::fill())
			.height(Size::px(48.0))
			.cont()
			.clickable()
			.on_press(move |_| {
				is_open2.toggle();
			})
			.child(ico)
			.child(
				segment(title, 1.0)
					.height(Size::fill())
					.main_align(Alignment::Center),
			)
			.child(arrow);

		rect()
			.width(Size::fill())
			.background(theme.error_bg)
			.border(theme.border(theme.error))
			.color(theme.error)
			.corner_radius(theme.round)
			.child(header)
			.maybe(*is_open.read(), |this| {
				this.child(
					rect()
						.width(Size::fill())
						.center()
						.padding(theme.gap2)
						.border(border_top(theme.border, theme.error))
						.child(contents),
				)
			})
	}
}

pub fn get_res_err_title(error: &SerializableResolutionError) -> String {
	match error {
		SerializableResolutionError::PackageContext(req, ..) => format!("In package {req}"),
		SerializableResolutionError::FailedToPreload(..) => "Failed to preload package".to_string(),
		SerializableResolutionError::FailedToGetProperties(..) => {
			"Failed to get package properties".to_string()
		}
		SerializableResolutionError::NoValidVersionsFound(..) => {
			"No valid versions found".to_string()
		}
		SerializableResolutionError::ExtensionNotFulfilled(..) => {
			"Extension not fulfilled".to_string()
		}
		SerializableResolutionError::ExplicitRequireNotFulfilled(..) => {
			"Explicit require not fulfilled".to_string()
		}
		SerializableResolutionError::IncompatiblePackage(..) => "Incompatible package".to_string(),
		SerializableResolutionError::FailedToEvaluate(..) => {
			"Failed to evaluate package".to_string()
		}
		SerializableResolutionError::Misc(..) => "Miscellaneous error".to_string(),
	}
}

pub fn get_res_err_contents(error: &SerializableResolutionError) -> Element {
	match error {
		SerializableResolutionError::PackageContext(_, err) => get_res_err_contents(err),
		SerializableResolutionError::FailedToPreload(err) => err.to_string().into_element(),
		SerializableResolutionError::FailedToGetProperties(req, err) => paragraph()
			.span("Failed to get properties for package ")
			.child(PackageChip {
				req: req.clone(),
				error: true,
			})
			.span(": ")
			.span(err.to_string())
			.into_element(),
		SerializableResolutionError::NoValidVersionsFound(req, constraints) => paragraph()
			.span("No valid versions found for package ")
			.child(PackageChip {
				req: req.clone(),
				error: true,
			})
			.span(". Constraints: ")
			.child(constraints.iter().join(","))
			.into_element(),
		SerializableResolutionError::ExtensionNotFulfilled(req, ext) => {
			if let Some(req) = req {
				paragraph()
					.span("Extension ")
					.span(ext.to_string())
					.span(" not fulfilled for package ")
					.child(PackageChip {
						req: req.clone(),
						error: true,
					})
					.into_element()
			} else {
				format!("Extension {ext} not fulfilled").into_element()
			}
		}
		SerializableResolutionError::ExplicitRequireNotFulfilled(ext, req) => paragraph()
			.span("Explicit require ")
			.span(ext.to_string())
			.span(" not fulfilled for package ")
			.child(PackageChip {
				req: req.clone(),
				error: true,
			})
			.into_element(),
		SerializableResolutionError::IncompatiblePackage(req, incompats) => paragraph()
			.span("Package ")
			.child(PackageChip {
				req: req.clone(),
				error: true,
			})
			.span(" is incompatible with packages ")
			.child(incompats.iter().join(","))
			.into_element(),
		SerializableResolutionError::FailedToEvaluate(req, err) => paragraph()
			.span("Failed to evaluate package ")
			.child(PackageChip {
				req: req.clone(),
				error: true,
			})
			.span(": ")
			.span(err.to_string())
			.into_element(),
		SerializableResolutionError::Misc(err) => {
			format!("Miscellaneous error: {err}").into_element()
		}
	}
}
