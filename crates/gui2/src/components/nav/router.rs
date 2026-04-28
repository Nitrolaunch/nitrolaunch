use gpui::prelude::*;
use gpui::*;

use crate::event::AppEvent;
use crate::pages::home::HomePage;
use crate::state::AppState;

pub struct Router {
	app_state: AppState,
	route: Page,
	home_page: Entity<HomePage>,
}

impl Router {
	pub fn new(app_state: AppState, window: &Window, cx: &mut Context<Self>) -> Self {
		let mut rx = app_state.subscribe();
		cx.spawn(async move |this, cx| {
			loop {
				if let Ok(AppEvent::RouteChanged(route)) = rx.recv().await {
					let _ = this.update(cx, move |this, cx| {
						this.route = route;
						cx.notify();
					});
				}
			}
		})
		.detach();

		Self {
			home_page: cx.new(|cx| HomePage::new(app_state.clone(), window, cx)),
			app_state,
			route: Page::Home,
		}
	}
}

impl Render for Router {
	fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
		let route = match &self.route {
			Page::Home => self.home_page.clone().into_any_element(),
			Page::Packages => (gpui_rsx::rsx! { <div></div> }).into_any_element(),
			Page::Plugins => (gpui_rsx::rsx! { <div></div> }).into_any_element(),
		};

		gpui_rsx::rsx! {
			<div id="router" size_full>
				{route}
			</div>
		}
	}
}

/// Page for the router
#[derive(Clone)]
pub enum Page {
	Home,
	Packages,
	Plugins,
}

impl Page {
	pub fn get_category(&self) -> PageCategory {
		match self {
			Self::Home => PageCategory::Home,
			Self::Packages => PageCategory::Packages,
			Self::Plugins => PageCategory::Plugins,
		}
	}
}

/// Category for pages
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PageCategory {
	Home,
	Packages,
	Plugins,
}

impl PageCategory {
	/// Gets the 'home page' for this category
	pub fn get_page(&self) -> Page {
		match self {
			Self::Home => Page::Home,
			Self::Packages => Page::Packages,
			Self::Plugins => Page::Plugins,
		}
	}
}
