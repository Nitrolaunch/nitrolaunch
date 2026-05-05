use std::collections::VecDeque;

/// Manages route and history
#[derive(Clone)]
pub struct Navigator {
	history: VecDeque<Page>,
	/// The index of the current page
	current: usize,
}

impl Navigator {
	pub fn new() -> Self {
		Self {
			history: VecDeque::from_iter(std::iter::once(Page::Home)),
			current: 0,
		}
	}

	pub fn navigate(&mut self, route: Page) {
		// If we are not at the end, replace the forward history with just the new route (we are making a new branch)
		self.history.truncate(self.current + 1);
		self.history.push_back(route);
        self.current = self.history.len() - 1;
	}

	pub fn route(&self) -> &Page {
		debug_assert!(!self.history.is_empty(), "History is empty");
		debug_assert!(
			self.current < self.history.len(),
			"History pointer is invalid"
		);

		self.history.get(self.current).unwrap()
	}

	pub fn back(&mut self) {
		if self.can_go_back() {
			self.current -= 1;
		}
	}

	pub fn forward(&mut self) {
		if self.can_go_forward() {
			self.current += 1;
		}
	}

	pub fn can_go_back(&self) -> bool {
		self.current > 0
	}

	pub fn can_go_forward(&self) -> bool {
		self.current < self.history.len() - 1
	}
}

/// Page for the router
#[derive(Clone, PartialEq, Eq, Debug)]
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_navigate_route() {
		let mut nav = Navigator::new();
		nav.navigate(Page::Packages);
		assert_eq!(nav.route(), &Page::Packages);
	}

	#[test]
	fn test_navigate_forward_backward() {
		let mut nav = Navigator::new();
		nav.navigate(Page::Packages);
		nav.back();
		assert_eq!(nav.route(), &Page::Home);
		nav.forward();
		assert_eq!(nav.route(), &Page::Packages);
	}
}
