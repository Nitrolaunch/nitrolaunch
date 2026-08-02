use std::time::Duration;

use nitrolaunch::{
	instance::update::manager::UpdateSettings,
	shared::{UpdateDepth, minecraft::VersionType, output::NoOp},
};

use crate::prelude::*;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchMinecraftVersions {
	back_state: Captured<BackState>,
}

impl FetchMinecraftVersions {
	pub fn new(back_state: BackState, include_snapshots: bool) -> Query<Self> {
		Query::new(
			FetchMinecraftVersionsKey { include_snapshots },
			Self {
				back_state: Captured(back_state),
			},
		)
		.stale_time(Duration::from_mins(30))
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchMinecraftVersionsKey {
	include_snapshots: bool,
}

impl QueryCapability for FetchMinecraftVersions {
	type Ok = Vec<String>;
	type Err = anyhow::Error;
	type Keys = FetchMinecraftVersionsKey;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let config = back_state.config().await?;

			let core = config
				.get_core(
					None,
					&UpdateSettings {
						depth: UpdateDepth::Shallow,
						offline_auth: false,
					},
					&back_state.client,
					&config.plugins,
					&back_state.paths,
					&mut NoOp,
				)
				.await?;

			let version_manifest = core
				.get_version_manifest(None, UpdateDepth::Shallow, &mut NoOp)
				.await?;

			if !keys.include_snapshots {
				Ok(version_manifest
					.manifest
					.versions
					.iter()
					.filter_map(|x| {
						if let VersionType::Release = &x.ty {
							Some(x.id.clone())
						} else {
							None
						}
					})
					.rev()
					.collect())
			} else {
				Ok(version_manifest.list.clone())
			}
		})
	}
}
