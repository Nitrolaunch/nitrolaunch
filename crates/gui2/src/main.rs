use freya::radio::use_init_radio_station;

use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};
use crate::state::{AppChannel, AppState};

mod components;
mod event;
mod pages;
mod prelude;
/// :O
mod secrets;
mod state;
mod util;

#[tokio::main]
async fn main() {
	let window = WindowConfig::new(app).with_size(1200.0, 900.0);
	let config = LaunchConfig::new().with_window(window);

	launch(config);
}

fn app() -> impl IntoElement {
	use_init_radio_station::<AppState, AppChannel>(|| AppState::new());

	let router = rect()
		.width(Size::fill())
		.height(Size::flex(1.0))
		.child(Router);

	rect()
		.width(Size::fill())
		.height(Size::fill())
		.background((35, 35, 35))
		.color(Color::WHITE)
		.child(NavBar)
		.child(router)
}
