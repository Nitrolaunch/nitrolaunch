use freya::{prelude::spawn, query::QueriesStorage};

use crate::{components::instance::running_instances::FetchRunningInstances, pages::home::FetchItems};

/// Backend dependency that can be invalidated
pub enum BackDependency {
	Config,
	RunningInstances,
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
		}
	}
}
