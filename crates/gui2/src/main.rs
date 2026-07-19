#![cfg_attr(
	all(not(debug_assertions), target_os = "windows"),
	windows_subsystem = "windows"
)]

use freya::radio::{use_init_radio_station, use_radio};
use nitrolaunch::shared::output::NitroOutput;
use tokio::runtime::Builder;
use tokio::sync::broadcast;

use crate::components::dialog::tip::Tips;
use crate::components::dialog::toast::Toast;
use crate::components::footer::Footer;
use crate::pages::config::ConfigPage;
use crate::pages::settings::SettingsPage;
use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};
use crate::state::{BackEvent, BackState, FrontChannel, FrontState};
use crate::theme::ThemeDeser;
use crate::util::Shared;

mod components;
mod data;
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

fn main() {
	let rt = Builder::new_multi_thread().enable_all().build().unwrap();
	let _rt = rt.enter();

	let (event_tx, event_rx) = broadcast::channel(100);
	let back_state = rt.block_on(BackState::new(event_tx)).unwrap();

	let window = WindowConfig::new(move || app(back_state.clone(), event_rx.resubscribe()))
		.with_size(1400.0, 900.0)
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
		let back_state = use_consume::<BackState>();

		let front_state2 = front_state.clone();
		let back_state2 = back_state.clone();
		let radio = use_radio(FrontChannel::ThemeConfig);
		use_side_effect(move || {
			radio.read();
			let data = back_state2.data();
			let available_themes = back_state2.themes();
			back_state.output().debug("Applying theme".into());

			let mut theme = ThemeDeser::dark();
			for new_theme in data.base_theme.into_iter().chain(data.overlay_themes) {
				if new_theme == "light" {
					theme = theme.merge(ThemeDeser::light());
				} else if let Some(data) = available_themes.iter().find(|x| x.id == new_theme) {
					dbg!(&data.settings);
					if let Ok(new_theme) = serde_json::from_str::<ThemeDeser>(&data.settings) {
						theme = theme.merge(new_theme);
					}
				}
			}
			front_state2.write().set_theme(theme.into());
			back_state.output().debug("Theme applied".into());
		});

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
			.font_size(theme.font)
			.child(NavBar { show_sidebar })
			.child(Tips)
			.child(view)
			.child(Footer)
			.child(ConfigPage)
			.child(SettingsPage)
	}
}
