use freya::radio::use_init_radio_station;
use tokio::sync::broadcast;

use crate::components::footer::Footer;
use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};
use crate::state::{BackEvent, BackState, FrontChannel, FrontState};
use crate::util::Shared;

mod components;
mod dependency;
mod icons;
mod instance_manager;
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
	let (event_tx, event_rx) = broadcast::channel(100);
	let back_state = BackState::new(event_tx).await.unwrap();

	let window = WindowConfig::new(move || app(back_state.clone(), event_rx.resubscribe()))
		.with_size(1200.0, 900.0)
		.with_title("Nitrolaunch")
		.with_decorations(false)
		.with_app_id("Nitrolaunch");
	let config = LaunchConfig::new().with_window(window);

	launch(config);
}

fn app(back_state: BackState, event_rx: broadcast::Receiver<BackEvent>) -> impl IntoElement {
	let station = use_init_radio_station::<(), FrontChannel>(|| ());
	use_provide_context(|| Shared::new(FrontState::new(station, event_rx)));
	use_provide_context(|| back_state);

	App
}

#[derive(PartialEq)]
struct App;

impl Component for App {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		use_radio(FrontChannel::Theme).read();

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
