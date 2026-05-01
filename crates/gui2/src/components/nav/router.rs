use crate::{pages::home::HomePage, prelude::*};

#[derive(PartialEq)]
pub struct Router;

impl Component for Router {
	fn render(&self) -> impl IntoElement {
		rect()
			.width(Size::fill())
			.height(Size::fill())
			.child(HomePage)
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
