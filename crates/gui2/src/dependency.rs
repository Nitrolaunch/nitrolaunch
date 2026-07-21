use freya::{prelude::spawn, query::QueriesStorage};

use crate::ops::{account::FetchAccounts, instance::FetchItems, launch::FetchRunningInstances};

/// Backend dependency that can be invalidated
#[derive(Clone)]
pub enum BackDependency {
	Config,
	RunningInstances,
	Accounts,
}

impl BackDependency {
	/// Invalidates this dependency across the app
	pub fn invalidate(&self) {
		match self {
			Self::Config => {
				spawn(QueriesStorage::<FetchItems>::invalidate_all());
			}
			Self::RunningInstances => {
				spawn(QueriesStorage::<FetchRunningInstances>::invalidate_all());
			}
			Self::Accounts => {
				spawn(QueriesStorage::<FetchAccounts>::invalidate_all());
			}
		}
	}
}
