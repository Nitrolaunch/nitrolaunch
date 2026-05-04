use crate::{components::nav::router::PageCategory, prelude::*};

pub mod router;

#[derive(PartialEq)]
pub struct NavBar;

impl Component for NavBar {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let left = rect()
			.height(Size::fill())
			.width(Size::percent(100.0 / 3.0));

		let center = rect()
			.height(Size::fill())
			.width(Size::percent(100.0 / 3.0))
			.horizontal()
			.child(PageButton {
				category: PageCategory::Home,
			})
			.child(PageButton {
				category: PageCategory::Packages,
			})
			.child(PageButton {
				category: PageCategory::Plugins,
			});

		let right = rect()
			.width(Size::percent(100.0 / 3.0))
			.width(Size::percent(100.0 / 3.0));

		rect()
			.width(Size::fill())
			.height(Size::px(theme.navbar_height))
			.horizontal()
			.background(theme.navbar)
			.child(left)
			.child(center)
			.child(right)
	}
}

#[derive(PartialEq)]
struct PageButton {
	category: PageCategory,
}

impl Component for PageButton {
	fn render(&self) -> impl IntoElement {
		let mut state = use_radio(AppChannel::Route);
		let theme = use_theme();

		let title = match self.category {
			PageCategory::Home => "Home",
			PageCategory::Packages => "Packages",
			PageCategory::Plugins => "Plugins",
		};
		let ico = match self.category {
			PageCategory::Home => "home",
			PageCategory::Packages => "honeycomb",
			PageCategory::Plugins => "jigsaw",
		};

		let (fg, bg) = if state.read().route().get_category() == self.category {
			(theme.primary, theme.primary_bg)
		} else {
			(theme.disabled, theme.navbar)
		};

		let page = self.category.get_page();

		rect()
			.height(Size::fill())
			.width(Size::percent(100.0 / 3.0))
			.margin(3.0)
			.child(
				Button::new()
					.child(
						rect()
							.cont()
							.child(icon(ico).width(Size::px(16.0)).height(Size::px(16.0)))
							.child(title),
					)
					.width(Size::fill())
					.background(bg)
					.hover_background(bg)
					.color(fg)
					.border_fill(fg)
					.on_press(move |_| state.write().navigate(page.clone())),
			)
	}
}
