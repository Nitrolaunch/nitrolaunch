use std::sync::Arc;

use anyhow::Context;
use nitrolaunch::{
	io::logging::{get_log_file_path, get_log_files},
	plugin_crate::hook::hooks,
	shared::{
		id::InstanceID,
		output::{MessageContents, NitroOutput},
	},
};

use crate::{
	dependency::BackDependency,
	ops::{QueryOption, task::Task},
	prelude::*,
	simple_mutation, simple_query,
};

simple_query!(
	name = FetchGlobalLogs,
	ok = Arc<[String]>,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(back_state.0.clone(), async move {
			let logs = get_log_files(&back_state.paths, "gui")?;
			Ok(logs.into_iter().map(|x| x.file_name().unwrap_or_default().to_string_lossy().to_string()).collect())
		})
	}
);

simple_query!(
	name = FetchGlobalLog,
	ok = Arc<str>,
	err = anyhow::Error,
	keys = QueryOption<String>,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let log = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let path = if let Some(log) = log.0 {
				get_log_file_path(&back_state.paths, "gui", &log)
			} else {
				get_log_files(&back_state.paths, "gui")?.first().context("No log available")?.to_owned()
			};
			let contents = std::fs::read_to_string(path)?;
			Ok(contents.into())
		})
	}
);

simple_mutation!(
	name = ShowDirectory,
	ok = (),
	err = anyhow::Error,
	keys = ShowDirectoryOption,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let mut o = back_state.output();
			o.set_task(Task::Opening);
			let dir = match keys {
				ShowDirectoryOption::Data => back_state.paths.data.clone(),
				ShowDirectoryOption::Config => back_state.paths.config.clone(),
				ShowDirectoryOption::Instance(instance_id) => {
					let config = back_state.config().await?;
					let instance = config
						.instances
						.get(&InstanceID::from(instance_id))
						.context("Instance does not exist")?;

					let Some(dir) = instance.dir() else {
						return Ok(());
					};

					dir.to_path_buf()
				}
			};

			o.finish_task();
			tokio::task::spawn_blocking(move || {
				if let Err(e) = open::that(dir) {
					o.debug(MessageContents::Error(format!("Failed to show dir: {e:?}")));
				}
			});

			Ok(())
		})
	}
);

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ShowDirectoryOption {
	Data,
	Config,
	Instance(String),
}

simple_mutation!(
	name = Sync,
	ok = (),
	err = anyhow::Error,
	keys = (),
	fn run(&self, _keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(back_state.0.clone(), async move {
			let back_state2 = back_state.clone();
			let task = async move {
				let mut output = back_state2.output();
				output.set_task(Task::Sync);
				let results = back_state2
					.plugins
					.call_hook(hooks::Sync, &(), &back_state2.paths, &mut output)
					.await?;
				results.all_results(&mut output).await?;
				back_state2.invalidate(BackDependency::Items);
				Ok(())
			};
			back_state.register_task(Task::Sync, tokio::spawn(task));

			Ok(())
		})
	}
);
