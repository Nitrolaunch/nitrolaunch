use anyhow::anyhow;
use gpui_component::{Root, Theme, ThemeSet};
use std::borrow::Cow;
use std::rc::Rc;

use crate::prelude::*;

use crate::components::nav::{NavBar, router::Router};

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
	let app = gpui_platform::application().with_assets(Assets);

	app.run(move |cx| {
		gpui_component::init(cx);
		let mut themes: ThemeSet = serde_json::from_slice(include_bytes!("../theme.json")).unwrap();
		Theme::global_mut(cx).apply_config(&Rc::new(themes.themes.remove(0)));

		let inter = include_bytes!("assets/inter.regular.ttf");
		if let Err(e) = cx.text_system().add_fonts(vec![Cow::Borrowed(inter)]) {
			eprintln!("Failed to add Inter font: {e}");
		}

		cx.spawn(async move |cx| {
			let window = WindowOptions {
				// window_min_size: Some(gpui::Size {
				// 	width: px(1000.0),
				// 	height: px(700.0),
				// }),
				kind: WindowKind::Floating,
				titlebar: Some(TitlebarOptions {
					title: Some("Nitrolaunch".into()),
					..Default::default()
				}),
				..Default::default()
			};

			cx.open_window(window, |window, cx| {
				let view = cx.new(|cx| HelloWorld::new(window, cx));

				cx.new(|cx| Root::new(view, window, cx))
			})
			.expect("Failed to open window");
		})
		.detach();
	});
}

#[derive(rust_embed::RustEmbed)]
#[folder = "./src/assets"]
struct Assets;

impl AssetSource for Assets {
	fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
		if path.is_empty() {
			return Ok(None);
		}

		Self::get(path)
			.map(|f| Some(f.data))
			.ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
	}

	fn list(&self, path: &str) -> Result<Vec<SharedString>> {
		Ok(Self::iter()
			.filter_map(|p| p.starts_with(path).then(|| p.into()))
			.collect())
	}
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
		rsx! {
			<v_flex size_full text_size={px(16.0)} font={font("Rubik")}>
				{self.nav_bar.clone()}
				<sect w_full>{self.router.clone()}</sect>
			</v_flex>
		}
	}
}
