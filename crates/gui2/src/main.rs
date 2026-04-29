use gpui::*;
use gpui_component::*;

use crate::{
	components::nav::{NavBar, router::Router},
	state::AppState,
};

mod components;
mod event;
mod pages;
/// :O
mod secrets;
mod state;
mod util;

#[tokio::main]
async fn main() {
	println!("{}", gpui::guess_compositor());
	let app = gpui_platform::application();
	app.run(move |cx| {
		gpui_component::init(cx);

		cx.spawn(async move |cx| {
			cx.open_window(WindowOptions::default(), |window, cx| {
				let view = cx.new(|cx| HelloWorld::new(window, cx));

				cx.new(|cx| Root::new(view, window, cx))
			})
			.expect("Failed to open window");
		})
		.detach();
	});
}

struct HelloWorld {
	app_state: AppState,
	nav_bar: Entity<NavBar>,
	router: Entity<Router>,
}

impl HelloWorld {
	fn new(window: &Window, cx: &mut Context<Self>) -> Self {
		let app_state = AppState::new();
		Self {
			nav_bar: cx.new(|cx| NavBar::new(app_state.clone(), window, cx)),
			router: cx.new(|cx| Router::new(app_state.clone(), window, cx)),
			app_state,
		}
	}
}

impl Render for HelloWorld {
	fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
		gpui_rsx::rsx! {
			<div size_full flex flex_col>
				{self.nav_bar.clone()}
				<div w_full flex_grow flex_basis={px(0.0)}>{self.router.clone()}</div>
			</div>
		}
	}
}
