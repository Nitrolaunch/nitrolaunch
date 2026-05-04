use freya::radio::use_init_radio_station;

use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};
use crate::state::{AppChannel, AppState};

mod components;
mod event;
mod icons;
mod pages;
mod prelude;
/// :O
mod secrets;
mod state;
mod theme;
mod util;

#[tokio::main]
async fn main() {
	let window = WindowConfig::new(app)
		.with_size(1200.0, 900.0)
		.with_title("Nitrolaunch")
		.with_decorations(false)
		.with_app_id("Nitrolaunch");
	let config = LaunchConfig::new().with_window(window);

	launch(config);
}

fn app() -> impl IntoElement {
	use_init_radio_station::<AppState, AppChannel>(|| AppState::new());
	let theme = use_theme();

	let router = rect()
		.width(Size::fill())
		.height(Size::flex(1.0))
		.child(Router);

	rect()
		.width(Size::fill())
		.height(Size::fill())
		.background(theme.bg)
		.color(theme.fg)
		.font_size(14.0)
		.child(NavBar)
		.child(router)
}
