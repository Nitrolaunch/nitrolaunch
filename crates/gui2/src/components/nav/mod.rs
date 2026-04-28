use gpui::prelude::*;
use gpui::*;
use gpui_component::Selectable;
use gpui_component::button::Button;

use crate::components::nav::router::{Page, PageCategory};
use crate::event::AppEvent;
use crate::state::AppState;

pub mod router;

pub struct NavBar {
	app_state: AppState,
	tab: PageCategory,
}

impl NavBar {
	pub fn new(app_state: AppState, _: &Window, cx: &mut Context<Self>) -> Self {
		let mut rx = app_state.subscribe();
		cx.spawn(async move |this, cx| {
			loop {
				if let Ok(AppEvent::RouteChanged(route)) = rx.recv().await {
					let _ = this.update(cx, move |this, cx| {
						this.tab = route.get_category();
						cx.notify();
					});
				}
			}
		})
		.detach();

		Self {
			app_state,
			tab: PageCategory::Home,
		}
	}

	pub fn route(&mut self, route: Page, cx: &mut Context<Self>) {
		self.tab = route.get_category();
		self.app_state.set_route(route);
		cx.notify();
	}
}

impl Render for NavBar {
	fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
		gpui_rsx::rsx! {
			<div id="navbar" grid grid_cols={3} w_full h={px(22.0)} absolute top_0 left_0>
				<div id="navbar-left">{"Nitrolaunch"}</div>
				<div id="navbar-center" grid grid_cols={3}>
					{navbar_button(PageCategory::Home, self.tab, cx.entity())}
					{navbar_button(PageCategory::Packages, self.tab, cx.entity())}
					{navbar_button(PageCategory::Plugins, self.tab, cx.entity())}
				</div>
				<div id="navbar-right"></div>
			</div>
		}
	}
}

fn navbar_button(
	tab: PageCategory,
	selected_tab: PageCategory,
	navbar: Entity<NavBar>,
) -> impl IntoElement {
	let title = match tab {
		PageCategory::Home => "Home",
		PageCategory::Packages => "Packages",
		PageCategory::Plugins => "Plugins",
	};
	let selected = tab == selected_tab;

	Button::new(ElementId::Name(SharedString::new(format!(
		"nav-tab-{title}"
	))))
	.label(title)
	.selected(selected)
	.on_click(move |_, _, cx| {
		navbar.update(cx, |nav_bar, cx| {
			nav_bar.route(tab.get_page(), cx);
		});
	})
}
