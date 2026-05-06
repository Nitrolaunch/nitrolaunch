use nitrolaunch::instance::tracking::RunningInstanceEntry;

use crate::prelude::*;

/// Only for the initial fetch. Events will be used afterwards.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchRunningInstances {
	back_state: Captured<BackState>,
}

impl FetchRunningInstances {
	pub fn new(back_state: BackState) -> Query<Self> {
		Query::new(
			(),
			Self {
				back_state: Captured(back_state),
			},
		)
	}
}

impl QueryCapability for FetchRunningInstances {
	type Ok = Vec<RunningInstanceEntry>;
	type Err = anyhow::Error;
	type Keys = ();

	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move { Ok(back_state.running_instances.get_running_instances().await) })
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KillInstance {
	id: String,
	account: Option<String>,
	back_state: Captured<BackState>,
}

impl KillInstance {
	pub fn new(id: String, account: Option<String>, back_state: BackState) -> Mutation<Self> {
		Mutation::new(Self {
			id,
			account,
			back_state: Captured(back_state),
		})
	}
}

impl MutationCapability for KillInstance {
	type Ok = ();
	type Err = anyhow::Error;
	type Keys = ();

	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let id = self.id.clone();
		let account = self.account.clone();
		let back_state = self.back_state.clone();

		query_spawn(async move {
			Ok(back_state
				.running_instances
				.kill(&id, account.as_deref())
				.await)
		})
	}
}
