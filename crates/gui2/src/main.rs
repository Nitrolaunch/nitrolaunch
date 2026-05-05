use freya::radio::use_init_radio_station;

use crate::components::footer::Footer;
use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};
use crate::state::{AppChannel, AppState};

mod components;
mod icons;
mod pages;
mod prelude;
mod routing;
/// :O
mod secrets;
mod state;
mod theme;
mod util;

#[tokio::main]
async fn main() {
	let app_state = AppState::new().await.unwrap();

	let window = WindowConfig::new(move || app(app_state.clone()))
		.with_size(1200.0, 900.0)
		.with_title("Nitrolaunch")
		.with_decorations(false)
		.with_app_id("Nitrolaunch");
	let config = LaunchConfig::new().with_window(window);

	launch(config);
}

fn app(app_state: AppState) -> impl IntoElement {
	use_init_radio_station::<AppState, AppChannel>(|| app_state);

	App
}

#[derive(PartialEq)]
struct App;

impl Component for App {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let show_sidebar = use_state(|| false);

		let router = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.child(Router::new());

		let sidebar = if *show_sidebar.read() {
			rect()
				.width(Size::px(theme.sidebar_width))
				.height(Size::fill())
				.background(theme.sidebar)
		} else {
			rect()
		};

		let view = rect()
			.width(Size::fill())
			.height(Size::flex(1.0))
			.flex()
			.horizontal()
			.child(sidebar)
			.child(router);

		rect()
			.width(Size::fill())
			.height(Size::fill())
			.flex()
			.background(theme.bg)
			.color(theme.fg)
			.font_size(14.0)
			.child(NavBar { show_sidebar })
			.child(view)
			.child(Footer)
	}
}
