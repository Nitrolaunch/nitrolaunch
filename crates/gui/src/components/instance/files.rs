use std::path::PathBuf;

use crate::{components::misc::number_indicator, ops::instance::FetchInstanceFiles, prelude::*};

#[derive(PartialEq)]
pub struct InstanceFilesView {
	pub id: String,
}

impl Component for InstanceFilesView {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let files = use_query(Query::new(
			self.id.clone(),
			FetchInstanceFiles::new(back_state.clone()),
		));
		let files = files.read().state().ok().cloned().unwrap_or_default();

		let save_count = files.saves.len();
		let saves = files.saves.into_iter().map(|x| {
			Item {
				icon: x.icon_path.map(|x| ImageSource::Path(PathBuf::from(x))),
				name: x.name,
			}
			.into_element()
		});
		let saves = ScrollView::new()
			.width(Size::fill())
			.height(Size::fill())
			.spacing(theme.gap)
			.children(saves);
		let left = rect()
			.width(Size::percent(50.0))
			.height(Size::fill())
			.padding(theme.gap2)
			.spacing(theme.gap2)
			.border(border_right(theme.border, theme.panel_border))
			.child(
				rect()
					.width(Size::fill())
					.horizontal()
					.spacing(theme.gap)
					.center()
					.child(icon("minecraft", 16.0))
					.child("Worlds")
					.child(number_indicator(save_count, &theme)),
			)
			.child(saves);
		let right = rect().width(Size::percent(50.0)).height(Size::fill());

		rect().expanded().child(left).child(right)
	}
}

#[derive(PartialEq)]
struct Item {
	name: String,
	icon: Option<ImageSource>,
}

impl Component for Item {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_hovered = use_state(|| false);

		let ico = if let Some(ico) = &self.icon {
			ImageViewer::new(ico.clone())
				.width(Size::px(32.0))
				.height(Size::px(32.0))
				.corner_radius(theme.round)
				.error_renderer(|e| {
					eprintln!("{e}");
					rect().into_element()
				})
				.into_element()
		} else {
			icon("box", 24.0).into_element()
		};
		let ico = rect()
			.width(Size::px(48.0))
			.height(Size::px(48.0))
			.center()
			.child(ico);

		rect()
			.width(Size::fill())
			.height(Size::px(48.0))
			.cont()
			.hover(is_hovered)
			.panel_colorway(&theme, *is_hovered.read(), false)
			.corner_radius(theme.round)
			.child(ico)
			.child(
				segment(self.name.clone(), 1.0)
					.height(Size::fill())
					.main_align(Alignment::Center),
			)
	}
}
