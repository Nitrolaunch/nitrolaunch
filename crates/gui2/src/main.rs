use freya::radio::use_init_radio_station;
use tokio::sync::broadcast;

use crate::components::dialog::tip::Tips;
use crate::components::dialog::toast::Toast;
use crate::components::footer::Footer;
use crate::pages::config::ConfigPage;
use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};
use crate::state::{BackEvent, BackState, FrontChannel, FrontState};
use crate::util::Shared;

mod components;
mod dependency;
mod icons;
mod instance_manager;
mod ops;
mod output;
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
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Theme);

		let show_sidebar = use_state(|| false);

		let router = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.child(Router::new());

		let front_state2 = front_state.clone();
		let front_state3 = front_state.clone();
		let front_state4 = front_state.clone();

		let sidebar = if *show_sidebar.read() {
			rect()
				.width(Size::px(theme.sidebar_width))
				.height(Size::fill())
				.background(theme.sidebar)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::WHITE)
						.on_press(move |_| {
							front_state.write().toast(Toast::info(
								"Info",
								Some("Lorem ipsum dolor sit amet adipiscing sdofijsdfoisjdfoisjdoflij".into_element()),
							));
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::GREEN)
						.on_press(move |_| {
							front_state2.write().toast(Toast::success("Success"));
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::YELLOW)
						.on_press(move |_| {
							front_state3.write().toast(Toast::warning(
								"Warning",
								Some("Lorem ipsum dolor sit amet adipiscing sdofijsdfoisjdfoisjdoflij".into_element()),
							));
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::RED)
						.on_press(move |_| {
							front_state4.write().toast(Toast::error(
								"Error",
								Some("Lorem ipsum dolor sit amet adipiscing sdofijsdfoisjdfoisjdoflij".into_element()),
							));
						}),
				)
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
			.child(Tips)
			.child(view)
			.child(Footer)
			.child(ConfigPage)
	}
}
