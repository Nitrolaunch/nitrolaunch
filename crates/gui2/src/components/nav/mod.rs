use crate::{
	components::{dialog::toast::Toasts, input::tabs::TopTabs},
	prelude::*,
	routing::PageCategory,
	state::ModalType,
};

pub mod page_buttons;
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
		let selected_category = use_reactive(&front_state.read().route().get_category());
		let selected_category2 = selected_category.clone();
		let front_state2 = front_state.clone();
		use_side_effect(move || {
			front_state2
				.write()
				.navigate(selected_category2.read().get_page());
		});

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

		let front_state2 = front_state.clone();
		let settings_button = icon_button("gear", &theme)
			.on_press(move |_| front_state2.write().set_modal(Some(ModalType::Settings)));

		let left = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.cont()
			.cross_align(Alignment::Center)
			.padding(3.0)
			.child(rect().margin(3.0).child(menu_button))
			.child(rect().margin(3.0).child(back_button))
			.child(rect().margin(3.0).child(forward_button))
			.child(rect().margin(3.0).child(settings_button));

		let buttons = TopTabs::new(selected_category)
			.child(SelectOption::new(PageCategory::Home, "Home", Some("home")))
			.child(SelectOption::new(
				PageCategory::Packages,
				"Packages",
				Some("honeycomb"),
			));
		let center = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.horizontal()
			.flex()
			.child(buttons);

		let right = rect()
			.height(Size::fill())
			.width(Size::flex(1.0))
			.padding(Gaps::new(3.0, 6.0, 3.0, 3.0))
			.cont()
			.center()
			.main_align(Alignment::End)
			.child(Toasts);

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
