use std::sync::Arc;

use crate::{
	components::console::{Console, ConsoleImpl},
	ops::instance::{FetchInstanceLogs, FetchInstanceOutput},
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
		let selected_log = use_state::<Option<String>>(|| None);
		let contents_query = use_query(FetchInstanceOutput::new(
			self.id.clone(),
			selected_log.read().clone(),
			back_state.clone(),
		));
		let logs = use_query(Query::new(
			self.id.clone(),
			FetchInstanceLogs::new(back_state.clone()),
		));

		let contents = contents_query
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default()
			.map(PtrEq);

		let logs = logs.read().state().ok().cloned().unwrap_or_default();

		let console = Impl {
			contents,
			log_files: PtrEq(logs),
			selected_log,
			is_loading: !contents_query.read().state().is_ok(),
		};

		Console { console }
	}
}

#[derive(PartialEq, Clone)]
struct Impl {
	contents: Option<PtrEq<str>>,
	log_files: PtrEq<[String]>,
	selected_log: State<Option<String>>,
	is_loading: bool,
}

impl ConsoleImpl for Impl {
	fn contents(&self) -> Option<Arc<str>> {
		self.contents.as_ref().map(|x| x.0.clone())
	}

	fn is_loading(&self) -> bool {
		self.is_loading
	}

	fn get_log_files(&self) -> impl Iterator<Item = &String> {
		self.log_files.0.iter()
	}

	fn get_log_file(&self) -> Option<String> {
		self.selected_log.read().clone()
	}

	fn set_log_file(&self, file: Option<String>) {
		self.selected_log.clone().set(file);
	}

	fn input_fn(&self) -> Option<impl Fn(String) + 'static> {
		Some(|text| {
			println!("{text}");
		})
	}
}
