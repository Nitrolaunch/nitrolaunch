use dioxus::prelude::*;

use crate::{
	components::icon::{Icon, IconType},
	Route,
};

/// Navigation bar at the top of the page
#[component]
pub fn NavBar() -> Element {
	rsx! {
		div {
			id: "navbar-gap"
		}
		div {
			id: "navbar",
			div {
				class: "split3 fullwidth"
			}
			div {
				class: "split3 fullwidth"
			}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
struct NavBarButtonProps<F: Fn(&Route) -> bool + 'static> {
	icon: IconType,
	text: String,
	route: Route,
	route_match_fn: F,
	color: String,
	background_color: String,
}

/// Button for a main page in the nav bar
#[component]
fn NavBarButton<F: Fn(&Route) -> bool + 'static>(props: NavBarButtonProps<F>) -> Element {
	let route = use_route::<Route>();
	let nav = navigator();

	let mut is_hovered = use_signal(|| false);
	let is_selected = use_memo(move || (props.route_match_fn)(&route));

	let color2 = props.color.clone();
	let color = use_memo(move || {
		if *is_selected.read() {
			color2
		} else {
			"var(--fg)".into()
		}
	});
	let border_color = use_memo(move || {
		if *is_selected.read() || *is_hovered.read() {
			props.color
		} else {
			"var(--fg)".into()
		}
	});
	let background_color = use_memo(move || {
		if *is_selected.read() {
			props.background_color
		} else {
			"var(--bg)".into()
		}
	});

	let selected_class = use_memo(|| if *is_selected.read() { "selected" } else { "" });

	rsx! {
		div {
			class: "cont link navbar-button bubble-hover {selected_class}",
			style: "color:{color};background-color:{background_color};border-color:{border_color}",
			onmouseenter: |_| is_hovered.set(true),
			onmouseleave: |_| is_hovered.set(false),
			onclick: move |_| {
				nav.push(props.route);
			},
			Icon { icon: props.icon, size: "1rem" },
			div { class: "navbar-button-text", {props.text} }
		}
	}
}
