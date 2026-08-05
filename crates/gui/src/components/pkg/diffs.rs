use nitrolaunch::shared::pkg::PackageDiff;

use crate::{
	components::{
		dialog::modal::{Modal, ModalButton},
		pkg::PackageChip,
	},
	prelude::*,
	state::BackEvent,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct PackageDiffsModal {
	pub diffs: PtrEq<[PackageDiff]>,
}

impl Component for PackageDiffsModal {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();

		let diffs = ScrollView::new()
			.expanded()
			.children(self.diffs.0.iter().map(|x| diff(x, &theme).into_element()));

		let contents = rect().expanded().padding(theme.gap2).child(diffs);

		let front_state2 = front_state.clone();
		let back_state2 = back_state.clone();
		let back_state3 = back_state.clone();
		Modal::new("Confirm Changes".into(), "honeycomb".into())
			.on_close(move |_| {
				let _ = back_state2
					.event_tx
					.send(BackEvent::ConfirmYesNoPrompt { yes: false });
				front_state.write().set_modal(None);
			})
			.maybe_child(true, || contents)
			.cancel_button()
			.button(ModalButton {
				title: "Confirm".into(),
				icon: "check".into(),
				on_click: EventHandler::from(move |_| {
					let _ = back_state3
						.event_tx
						.send(BackEvent::ConfirmYesNoPrompt { yes: true });
					front_state2.write().set_modal(None);
				}),
				active: true,
			})
	}
}

fn diff(diff: &PackageDiff, theme: &Theme) -> impl IntoElement {
	let (ico, fg, bg) = match diff {
		PackageDiff::Added(..) | PackageDiff::ManyAdded(..) => {
			("plus", theme.success, theme.success_bg)
		}
		PackageDiff::Removed(..) | PackageDiff::ManyRemoved(..) => {
			("minus", theme.error, theme.error_bg)
		}
		PackageDiff::VersionChanged(..) => ("cycle", theme.warning, theme.error_bg),
	};

	let indicator = rect()
		.width(Size::px(theme.input_height))
		.height(Size::px(theme.input_height))
		.center()
		.color(fg)
		.background(bg)
		.border(theme.border(fg))
		.corner_radius(CornerRadius::new(theme.round, 0.0, 0.0, theme.round))
		.child(icon(ico, 16.0));

	let contents = match diff {
		PackageDiff::Added(req) | PackageDiff::Removed(req) => PackageChip {
			req: req.clone(),
			error: false,
		}
		.into_element(),
		PackageDiff::VersionChanged(req, from, to) => rect()
			.horizontal()
			.spacing(theme.gap)
			.child(PackageChip {
				req: req.clone(),
				error: false,
			})
			.child(label().text(from.clone()).color(theme.warning))
			.child(icon("arrow_right", 16.0))
			.child(label().text(to.clone()).color(theme.success))
			.into_element(),
		PackageDiff::ManyAdded(count) => format!("Add {count} packages").into_element(),
		PackageDiff::ManyRemoved(count) => format!("Remove {count} packages").into_element(),
	};

	rect()
		.width(Size::fill())
		.height(Size::px(theme.input_height))
		.padding(theme.gap)
		.corner_radius(theme.round)
		.horizontal()
		.flex()
		.cross_align(Alignment::Center)
		.border(Border {
			fill: theme.panel_border,
			width: BorderWidth {
				top: theme.border,
				right: theme.border,
				bottom: theme.border,
				left: 0.0,
			},
			alignment: BorderAlignment::Inner,
		})
		.child(indicator)
		.child(
			segment(contents, 1.0)
				.height(Size::fill())
				.main_align(Alignment::Center)
				.padding(Gaps::new(0.0, theme.gap, 0.0, theme.gap)),
		)
}
