use freya::{prelude::spawn, query::QueriesStorage};

use crate::ops::{
	account::FetchAccounts, instance::FetchInstanceConfig, launch::FetchRunningInstances,
	packages::FetchInstanceLockfile,
};

/// Backend dependency that can be invalidated
#[derive(Clone)]
pub enum BackDependency {
	RunningInstances,
	Accounts,
	InstanceContent(String),
}

impl BackDependency {
	/// Invalidates this dependency across the app
	pub fn invalidate(&self) {
		match self {
			Self::RunningInstances => {
				spawn(QueriesStorage::<FetchRunningInstances>::try_invalidate_all());
			}
			Self::Accounts => {
				spawn(QueriesStorage::<FetchAccounts>::try_invalidate_all());
			}
			Self::InstanceContent(id) => {
				spawn(QueriesStorage::<FetchInstanceConfig>::try_invalidate_matching(id.clone()));
				spawn(QueriesStorage::<FetchInstanceLockfile>::try_invalidate_matching(id.clone()));
			}
		}
	}
}
