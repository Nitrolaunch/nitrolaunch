use std::time::Duration;

use nitrolaunch::{
	config_crate::ConfigKind,
	plugin_crate::{
		control::Control,
		hook::hooks::{
			AccountTypeInfo, AddAccountTypes, AddInstanceConfigControls,
			AddInstanceConfigControlsArg, AddSupportedLoaders, GetLoaderVersions,
			GetLoaderVersionsArg,
		},
	},
	shared::{loaders::Loader, output::NoOp},
};

use crate::{prelude::*, simple_query, util::PtrEq};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchSupportedLoaders {
	back_state: Captured<BackState>,
}

impl FetchSupportedLoaders {
	pub fn new(back_state: BackState) -> Query<Self> {
		Query::new(
			(),
			Self {
				back_state: Captured(back_state),
			},
		)
		.stale_time(Duration::from_secs(30))
	}
}

impl QueryCapability for FetchSupportedLoaders {
	type Ok = Vec<Loader>;
	type Err = anyhow::Error;
	type Keys = ();

	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let results = back_state
				.plugins
				.call_hook(AddSupportedLoaders, &(), &back_state.paths, &mut NoOp)
				.await?;

			results.flatten_all_results(&mut NoOp).await
		})
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchLoaderVersions {
	back_state: Captured<BackState>,
}

impl FetchLoaderVersions {
	pub fn new(
		back_state: BackState,
		loader: Loader,
		minecraft_version: Option<String>,
	) -> Query<Self> {
		Query::new(
			FetchLoaderVersionsKey {
				loader,
				minecraft_version,
			},
			Self {
				back_state: Captured(back_state),
			},
		)
		.stale_time(Duration::from_mins(15))
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchLoaderVersionsKey {
	loader: Loader,
	minecraft_version: Option<String>,
}

impl QueryCapability for FetchLoaderVersions {
	type Ok = Vec<String>;
	type Err = anyhow::Error;
	type Keys = FetchLoaderVersionsKey;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(async move {
			let Some(minecraft_version) = keys.minecraft_version else {
				return Ok(Vec::new());
			};

			let arg = GetLoaderVersionsArg {
				loader: keys.loader,
				minecraft_version,
			};
			let results = back_state
				.plugins
				.call_hook(GetLoaderVersions, &arg, &back_state.paths, &mut NoOp)
				.await?;

			results.flatten_all_results(&mut NoOp).await
		})
	}
}

simple_query!(
	name = FetchAccountTypes,
	ok = Vec<AccountTypeInfo>,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let mut o = back_state.output();
			back_state
				.plugins
				.call_hook(AddAccountTypes, &(), &back_state.paths, &mut o)
				.await?
				.flatten_all_results(&mut o)
				.await
		})
	}
);

simple_query!(
	name = FetchInstanceControls,
	ok = PtrEq<[Control]>,
	err = anyhow::Error,
	keys = FetchInstanceControlsKeys,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(async move {
			let mut o = back_state.output();
			let arg = AddInstanceConfigControlsArg {
				id: keys.id,
				kind: keys.ty,
				plugin: keys.config_plugin,
			};
			let mut out = Vec::new();
			let mut results = back_state
				.plugins
				.call_hook(AddInstanceConfigControls, &arg, &back_state.paths, &mut o)
				.await?;
			while let Ok(Some(result)) = results.next_result(&mut o).await {
				out.extend(result.controls);
			}
			Ok(PtrEq(out.into_iter().collect()))
		})
	}
);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FetchInstanceControlsKeys {
	pub id: Option<String>,
	pub ty: ConfigKind,
	pub config_plugin: Option<String>,
}
