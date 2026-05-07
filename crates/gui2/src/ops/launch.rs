use anyhow::Context;
use nitrolaunch::{
	instance::{
		launch::LaunchSettings,
		tracking::RunningInstanceEntry,
		update::{InstanceUpdateContext, manager::UpdateSettings},
	},
	io::lock::Lockfile,
	shared::{UpdateDepth, id::InstanceID},
};

use crate::{ops::MakeSend, prelude::*, secrets::get_ms_client_id};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LaunchInstance {
	back_state: Captured<BackState>,
}

#[derive(Clone, PartialEq, Hash)]
pub struct LaunchInstanceParams {
	pub id: String,
	pub account: Option<String>,
	pub offline: bool,
}

impl LaunchInstance {
	pub fn new(back_state: BackState) -> Mutation<Self> {
		Mutation::new(Self {
			back_state: Captured(back_state),
		})
	}
}

impl MutationCapability for LaunchInstance {
	type Ok = ();
	type Err = anyhow::Error;
	type Keys = LaunchInstanceParams;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let id = keys.id.clone();
		let account = keys.account.clone();
		let offline = keys.offline;
		let back_state = self.back_state.clone();

		let task = async move {
			let mut config = back_state.config().await?;
			let mut output = back_state.output();
			output.set_task(&format!("launch_instance_{id}"));
			output.set_instance(id.clone().into());

			if let Some(account) = account {
				let _ = config.accounts.choose_account(&account);
			}

			let core = config
				.get_core(
					Some(&get_ms_client_id()),
					&UpdateSettings {
						depth: UpdateDepth::Shallow,
						offline_auth: offline,
					},
					&back_state.client,
					&config.plugins,
					&back_state.paths,
					&mut output,
				)
				.await?;

			let instance = config
				.instances
				.get_mut(&InstanceID::from(id))
				.context("Instance does not exist")?;

			let settings = LaunchSettings {
				offline_auth: offline,
				pipe_stdin: false,
				quick_play: None,
			};

			let mut lock = Lockfile::open(&back_state.paths)?;
			let mut ctx = InstanceUpdateContext {
				packages: &config.packages,
				accounts: &mut config.accounts,
				plugins: &config.plugins,
				prefs: &config.prefs,
				paths: &back_state.paths,
				lock: &mut lock,
				client: &back_state.client,
				output: &mut output,
				core: &core,
			};

			let mut handle = instance
				.launch(settings, &mut ctx)
				.await
				.context("Failed to launch instance")?;

			handle.silence_output(true);
			output.finish_task();

			handle
				.wait(&config.plugins, &back_state.paths, &mut output)
				.await?;

			Ok(())
		};

		let task = unsafe { MakeSend::new(task) };
		self.back_state
			.register_task(&format!("launch_instance_{}", keys.id), tokio::spawn(task));

		async { Ok(()) }
	}
}

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
