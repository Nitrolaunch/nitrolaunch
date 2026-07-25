use std::{sync::Arc, time::Duration};

use crate::{
	components::console::{Console, ConsoleImpl},
	ops::{
		instance::{FetchInstanceLogs, FetchInstanceOutput},
		launch::WriteInstanceInput,
	},
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
		let write_input = use_mutation(Mutation::new(WriteInstanceInput::new(back_state.clone())));

		let back_state2 = back_state.clone();
		let id2 = self.id.clone();
		let contents_query2 = contents_query.clone();
		use_future(move || {
			let back_state2 = back_state2.clone();
			let id2 = id2.clone();
			let contents_query2 = contents_query2.clone();
			async move {
				let mut last_modified = None;
				loop {
					tokio::time::sleep(Duration::from_millis(100)).await;

					let Some(entry) = back_state2.running_instances.get_entry(&id2, None).await
					else {
						continue;
					};
					let Some(stdout_file) = &entry.stdout_file else {
						continue;
					};
					let path = back_state2.paths.internal.join("stdio").join(stdout_file);
					let Ok(metadata) = tokio::fs::metadata(&path).await else {
						continue;
					};

					let modified = metadata.modified().ok();
					if last_modified != modified {
						last_modified = modified;
						contents_query2.invalidate();
					}
				}
			}
		});

		let contents = contents_query
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default()
			.map(PtrEq);

		let logs = logs.read().state().ok().cloned().unwrap_or_default();

		let id = self.id.clone();
		let console = Impl {
			contents,
			log_files: PtrEq(logs),
			selected_log,
			is_loading: !contents_query.read().state().is_ok(),
			write_input: (move |command| {
				write_input.mutate((id.clone(), command));
			})
			.into(),
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
	write_input: EventHandler<String>,
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

	fn input_fn(&self) -> Option<impl Fn(String) -> bool + 'static> {
		None::<Box<dyn Fn(String) -> bool>>
		// Disabled until we can make it work
		// let write_input = self.write_input.clone();
		// Some(move |mut text: String| {
		// 	text.push('\n');
		// 	write_input.call(text);
		// 	true
		// })
	}
}
