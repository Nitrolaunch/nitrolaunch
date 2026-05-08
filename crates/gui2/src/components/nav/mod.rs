use crate::{prelude::*, routing::PageCategory};

pub mod router;

#[derive(PartialEq)]
pub struct NavBar {
	pub show_sidebar: State<bool>,
}

impl Component for NavBar {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Route);
		let theme = use_theme();

		let mut show_sidebar = self.show_sidebar.clone();
		let menu_button = icon_button("menu", &theme).on_press(move |_| show_sidebar.toggle());

		let front_state2 = front_state.clone();
		let mut back_button = icon_button("arrow_left", &theme)
			.on_press(move |_| front_state2.write().back())
			.enabled(front_state.read().can_go_back());
		if !front_state.read().can_go_back() {
			back_button = back_button.color(theme.disabled);
		}

		let front_state2 = front_state.clone();
		let mut forward_button = icon_button("arrow_right", &theme)
			.on_press(move |_| front_state2.write().forward())
			.enabled(front_state.read().can_go_forward());

		if !front_state.read().can_go_forward() {
			forward_button = forward_button.color(theme.disabled);
		}

		let left = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.cont()
			.cross_align(Alignment::Center)
			.padding(3.0)
			.child(rect().margin(3.0).child(menu_button))
			.child(rect().margin(3.0).child(back_button))
			.child(rect().margin(3.0).child(forward_button));

		let center = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.horizontal()
			.flex()
			.child(PageButton {
				category: PageCategory::Home,
			})
			.child(PageButton {
				category: PageCategory::Packages,
			})
			.child(PageButton {
				category: PageCategory::Plugins,
			});

		let right = rect().height(Size::fill()).width(Size::flex(1.0));

		rect()
			.width(Size::fill())
			.height(Size::px(theme.navbar_height))
			.horizontal()
			.background(theme.navbar)
			.flex()
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
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Route);
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

		let (fg, bg) = if front_state.read().route().get_category() == self.category {
			(theme.primary, theme.primary_bg)
		} else {
			(theme.disabled, theme.navbar)
		};

		let page = self.category.get_page();

		rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.margin(3.0)
			.child(
				Button::new()
					.child(rect().cont().child(icon(ico, 16.0)).child(title))
					.width(Size::fill())
					.background(bg)
					.hover_background(bg)
					.color(fg)
					.border_fill(fg)
					.on_press(move |_| front_state.write().navigate(page.clone())),
			)
	}
}
