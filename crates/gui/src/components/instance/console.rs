use std::sync::Arc;

use nitrolaunch::instance::operations::{
	LOCAL_INSTANCE_OUTPUT_UPDATE_RATE, REMOTE_INSTANCE_OUTPUT_UPDATE_RATE,
};

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
	pub is_remote: bool,
}

impl Component for InstanceConsole {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let selected_log = use_state::<Option<String>>(|| None);
		let char_count = use_state::<u64>(|| 0);
		let mut total_content = use_state::<Option<Arc<str>>>(|| None);
		let contents_query = use_query(FetchInstanceOutput::new(
			self.id.clone(),
			selected_log.read().clone(),
			*char_count.read(),
			back_state.clone(),
		));
		let logs = use_query(Query::new(
			self.id.clone(),
			FetchInstanceLogs::new(back_state.clone()),
		));
		let write_input = use_mutation(Mutation::new(WriteInstanceInput::new(back_state.clone())));

		let update_rate = if self.is_remote {
			REMOTE_INSTANCE_OUTPUT_UPDATE_RATE
		} else {
			LOCAL_INSTANCE_OUTPUT_UPDATE_RATE
		};
		let contents_query2 = contents_query.clone();
		use_future(move || async move {
			loop {
				tokio::time::sleep(update_rate).await;
				contents_query2.invalidate();
			}
		});
		let contents_query2 = contents_query.clone();
		let mut char_count2 = char_count.clone();
		use_side_effect(move || {
			let new_content = contents_query2
				.read()
				.state()
				.ok()
				.cloned()
				.flatten()
				.unwrap_or_default();
			let combined = total_content
				.peek()
				.as_ref()
				.unwrap_or(&Arc::default())
				.as_ref()
				.to_string() + &new_content;
			total_content.set(Some(combined.into()));
			*char_count2.write() += new_content.len() as u64;
		});

		let logs = logs.read().state().ok().cloned().unwrap_or_default();

		let id = self.id.clone();
		let console = Impl {
			contents: total_content.read().cloned().map(PtrEq),
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
