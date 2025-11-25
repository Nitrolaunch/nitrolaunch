use dioxus::{
	desktop::{Config, LogicalSize, WindowBuilder},
	prelude::*,
};

use views::Home;

use crate::components::navigation::navbar::NavBar;

/// Reusable components
mod components;
/// Views for different pages in the app
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
	#[layout(Layout)]
	#[route("/")]
	Home {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

fn main() {
	let window_cfg = WindowBuilder::new()
		.with_inner_size(LogicalSize::new(1200.0, 900.0)) // Set initial size (width, height)
		.with_title("Nitrolaunch");

	dioxus::LaunchBuilder::desktop()
		.with_cfg(Config::new().with_window(window_cfg))
		.launch(App);
}

#[component]
fn App() -> Element {
	rsx! {
		document::Link { rel: "icon", href: FAVICON }
		document::Link { rel: "stylesheet", href: MAIN_CSS }

		Router::<Route> {}
	}
}

#[component]
fn Layout() -> Element {
	rsx! {
		NavBar {},
		Outlet::<Route> {}
	}
}
