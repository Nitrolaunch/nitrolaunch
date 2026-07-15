use std::sync::Arc;

use crate::{
	components::console::{Console, ConsoleImpl},
	ops::instance::FetchInstanceOutput,
	prelude::*,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct InstanceConsole {
	pub id: String,
}

impl Component for InstanceConsole {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let contents = use_query(FetchInstanceOutput::new(self.id.clone(), back_state));

		let contents = contents
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default()
			.map(PtrEq);

		let console = Impl { contents };

		Console { console }
	}
}

#[derive(PartialEq)]
struct Impl {
	contents: Option<PtrEq<str>>,
}

impl ConsoleImpl for Impl {
	fn contents(&self) -> Option<Arc<str>> {
		self.contents.as_ref().map(|x| x.0.clone())
	}

	fn input_fn(&self) -> Option<impl Fn(String) + 'static> {
		Some(|text| {
			println!("{text}");
		})
	}
}
