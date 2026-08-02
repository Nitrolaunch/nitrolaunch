use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail};
use freya::query::QueriesStorage;
use nitrolaunch::{
	config_crate::ConfigKind,
	plugin_crate::{
		control::Control,
		hook::hooks::{
			AccountTypeInfo, AddAccountTypes, AddDropdownButtons, AddInstanceConfigControls,
			AddInstanceConfigControlsArg, AddSupportedLoaders, CustomAction, CustomActionArg,
			DropdownButton, DropdownButtonLocation, GetLoaderVersions, GetLoaderVersionsArg,
			GetPopup, GetPopupArg,
		},
	},
	shared::{
		loaders::Loader,
		output::{MessageContents, NitroOutput, NoOp},
	},
};

use crate::{
	ops::{instance::FetchItems, task::Task},
	prelude::*,
	simple_mutation, simple_query,
	state::{FrontState, ModalType},
	util::{PtrEq, Shared},
};

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

		query_spawn(back_state.0.clone(), async move {
			let results = back_state
				.plugins
				.call_hook(AddSupportedLoaders, &(), &back_state.paths, &mut NoOp)
				.await?;

			let mut out = results.flatten_all_results(&mut NoOp).await?;
			out.insert(0, Loader::Vanilla);
			Ok(out)
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

		query_spawn(back_state.0.clone(), async move {
			let mut o = back_state.output();
			o.set_task(Task::FetchLoaderVersions);
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

		query_spawn(back_state.0.clone(), async move {
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

		query_spawn(back_state.0.clone(), async move {
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

simple_query!(
	name = FetchDropdownButtons,
	ok = Vec<DropdownButton>,
	err = anyhow::Error,
	keys = DropdownButtonLocation,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let location = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let mut o = back_state.output();
			let results = back_state
				.plugins
				.call_hook(AddDropdownButtons, &(), &back_state.paths, &mut o)
				.await?
				.flatten_all_results(&mut o)
				.await?;

			Ok(results.into_iter().filter(|b| b.location == location).collect())
		})
	}
);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OpenCustomPopup {
	back_state: Captured<BackState>,
	front_state: Captured<Shared<FrontState>>,
}

impl OpenCustomPopup {
	pub fn new(back_state: BackState, front_state: Shared<FrontState>) -> Self {
		Self {
			back_state: Captured(back_state),
			front_state: Captured(front_state),
		}
	}
}

impl MutationCapability for OpenCustomPopup {
	type Ok = ();
	type Err = anyhow::Error;
	type Keys = OpenCustomPopupKeys;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		let popup = tokio::spawn(async move {
			let mut o = back_state.output();
			o.set_task(Task::Opening);
			o.show_toasts();
			let arg = GetPopupArg {
				id: keys.popup_id.clone(),
				payload: CustomActionArg {
					id: keys.popup_id,
					payload: serde_json::Value::Null,
					related_id: keys.related_id,
					control_state: serde_json::Map::new(),
				},
			};
			back_state
				.plugins
				.call_hook_on_plugin(GetPopup, &keys.plugin, &arg, &back_state.paths, &mut o)
				.await?
				.context("Popup does not exist")?
				.result(&mut o)
				.await
		});

		async move {
			let popup = match popup.await? {
				Ok(popup) => popup,
				Err(e) => {
					self.back_state
						.output()
						.display(MessageContents::Error(format!("{e:?}")));
					bail!("Failed to open popup");
				}
			};

			self.front_state
				.write()
				.set_modal(Some(ModalType::CustomPopup(PtrEq(Arc::new(popup)))));

			Ok(())
		}
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OpenCustomPopupKeys {
	pub plugin: String,
	pub popup_id: String,
	pub related_id: Option<String>,
}

#[rustfmt::skip]
simple_mutation!(
	name = RunCustomAction,
	ok = serde_json::Value,
	err = anyhow::Error,
	keys = RunCustomActionKeys,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let keys = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let mut o = back_state.output();
			o.set_task(Task::CustomAction);
			o.show_toasts();
			let arg = CustomActionArg {
				id: keys.action,
				payload: keys.params,
				related_id: keys.related_id,
				control_state: keys.control_state,
			};
			back_state
				.plugins
				.call_hook_on_plugin(CustomAction, &keys.plugin, &arg, &back_state.paths, &mut o)
				.await?
				.context("Custom action does not exist")?
				.result(&mut o)
				.await
		})
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		QueriesStorage::<FetchItems>::try_invalidate_all()
	}
);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RunCustomActionKeys {
	pub plugin: String,
	pub action: String,
	pub params: serde_json::Value,
	pub related_id: Option<String>,
	pub control_state: serde_json::Map<String, serde_json::Value>,
}
