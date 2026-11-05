use nitrolaunch::plugin_crate::hook::hooks::SearchServerRepository;
use nitrolaunch::shared::servers::{ServerInfo, ServerSearchParams};

use crate::{ops::query_spawn, prelude::*};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SearchServers {
	back_state: Captured<BackState>,
}

impl SearchServers {
	pub fn new(back_state: BackState) -> Self {
		Self {
			back_state: Captured(back_state),
		}
	}
}

#[derive(Clone, Default)]
pub struct ServerSearchResults {
	pub results: Vec<ServerInfo>,
	pub total_results: usize,
}

impl QueryCapability for SearchServers {
	type Ok = ServerSearchResults;
	type Err = anyhow::Error;
	type Keys = ServerSearchParams;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let params = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let mut output = back_state.output();
			let handles = back_state
				.plugins
				.call_hook(
					SearchServerRepository,
					&params,
					&back_state.paths,
					&mut output,
				)
				.await?;
			let results = handles.all_results(&mut output).await?;

			Ok(ServerSearchResults {
				results: results
					.iter()
					.flat_map(|result| result.results.clone())
					.collect(),
				total_results: results.iter().map(|result| result.total_results).sum(),
			})
		})
	}
}
