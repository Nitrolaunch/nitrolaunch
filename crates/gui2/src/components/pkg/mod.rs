use std::rc::Rc;

use itertools::Itertools;

use crate::{components::input::select::Selected, prelude::*, theme::Colorway};

#[derive(PartialEq)]
pub struct RepoSelector {
	pub repo: State<Option<String>>,
}

impl Component for RepoSelector {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let theme = use_theme();
		let repos = back_state.repos();

		let repo = self.repo.clone();

		InlineSelect::new(
			Selected::Single(self.repo.read().cloned()),
			Rc::new(move |selected| {
				repo.clone().set(selected.single().clone());
			}),
		)
		.fit()
		.child(SelectOption::new(None, "", Some("box")).tip("All repositories"))
		.children(
			repos
				.iter()
				.sorted_by_cached_key(|x| x.0.clone())
				.map(|(id, meta)| {
					let name = meta.name.as_deref().unwrap_or(id);
					let selected_bg = meta
						.color
						.as_deref()
						.and_then(Color::from_hex)
						.unwrap_or(theme.bg.into());

					let ico = meta
						.icon
						.as_deref()
						.map(|x| {
							svg(x.as_bytes().to_vec())
								.width(Size::px(16.0))
								.height(Size::px(16.0))
								.into_element()
						})
						.unwrap_or(icon("box", 16.0).into_element());
					SelectOption::new_custom_icon(Some(id.clone()), "", ico)
						.selected_colorway(Colorway::new(theme.bg, selected_bg, selected_bg))
						.tip(name)
				}),
		)
	}
}
