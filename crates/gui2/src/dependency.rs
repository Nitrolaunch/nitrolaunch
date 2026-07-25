use freya::{prelude::spawn, query::QueriesStorage};

use crate::ops::{account::FetchAccounts, launch::FetchRunningInstances};

/// Backend dependency that can be invalidated
#[derive(Clone)]
pub enum BackDependency {
	RunningInstances,
	Accounts,
}

impl BackDependency {
	/// Invalidates this dependency across the app
	pub fn invalidate(&self) {
		match self {
			Self::RunningInstances => {
				spawn(QueriesStorage::<FetchRunningInstances>::invalidate_all());
			}
			Self::Accounts => {
				spawn(QueriesStorage::<FetchAccounts>::invalidate_all());
			}
		}
	}
}
