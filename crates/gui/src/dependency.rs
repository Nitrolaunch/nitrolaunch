use freya::prelude::spawn;

use crate::ops::{
	account::FetchAccounts,
	instance::{FetchInstanceConfig, FetchItems},
	invalidate_all, invalidate_matching,
	launch::FetchRunningInstances,
	packages::{FetchInstanceAddons, FetchInstanceLockfile},
	plugins::FetchLocalPlugins,
};

/// Backend dependency that can be invalidated
#[derive(Clone)]
pub enum BackDependency {
	Items,
	RunningInstances,
	Accounts,
	InstanceContent(String),
	Plugins,
}

impl BackDependency {
	/// Invalidates this dependency across the app
	pub fn invalidate(&self) {
		match self {
			Self::Items => {
				spawn(invalidate_all::<FetchItems>());
			}
			Self::RunningInstances => {
				spawn(invalidate_all::<FetchRunningInstances>());
			}
			Self::Accounts => {
				spawn(invalidate_all::<FetchAccounts>());
			}
			Self::InstanceContent(id) => {
				spawn(invalidate_matching::<FetchInstanceConfig>(id.clone()));
				spawn(invalidate_matching::<FetchInstanceLockfile>(id.clone()));
				spawn(invalidate_matching::<FetchInstanceAddons>(id.clone()));
			}
			Self::Plugins => {
				spawn(invalidate_all::<FetchLocalPlugins>());
			}
		}
	}
}
