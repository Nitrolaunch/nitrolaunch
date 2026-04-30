use crate::prelude::*;

use crate::components::center;
use crate::components::nav::router::{Page, PageCategory};
use crate::state::AppState;

pub mod router;

pub struct NavBar {
	app_state: AppState,
	tab: PageCategory,
}

impl NavBar {
	pub fn new(app_state: AppState, _: &Window, cx: &mut Context<Self>) -> Self {
		let mut rx = app_state.subscribe_route();
		cx.spawn(async move |this, cx| {
			loop {
				if let Ok(()) = rx.changed().await {
					let route = rx.borrow().clone();
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
		rsx! {
			<div id="navbar" grid grid_cols={3} w_full h={px(40.0)} bg={cx.theme().title_bar}>
				<center id="navbar-left">{"Nitrolaunch"}</center>
				<center id="navbar-center">
					<div grid grid_cols={3} gap_1>
						{navbar_button(PageCategory::Home, self.tab, cx.entity())}
						{navbar_button(PageCategory::Packages, self.tab, cx.entity())}
						{navbar_button(PageCategory::Plugins, self.tab, cx.entity())}
					</div>
				</center>
				<center id="navbar-right"></center>
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
	let icon = match tab {
		PageCategory::Home => Icon::empty().path("icons/home.svg"),
		PageCategory::Packages => Icon::empty().path("icons/honeycomb.svg"),
		PageCategory::Plugins => Icon::empty().path("icons/jigsaw.svg"),
	};
	let selected = tab == selected_tab;

	Button::new(ElementId::Name(SharedString::new(format!(
		"nav-tab-{title}"
	))))
	.label(title)
	.icon(icon)
	.selected(selected)
	.on_click(move |_, _, cx| {
		navbar.update(cx, |nav_bar, cx| {
			nav_bar.route(tab.get_page(), cx);
		});
	})
}
