use anyhow::Context;
use freya::query::QueriesStorage;
use itertools::Itertools;
use nitrolaunch::{
	plugin::{PluginManager, install::get_verified_plugins},
	plugin_crate::plugin::PluginMetadata,
};

use crate::{ops::task::Task, prelude::*, simple_mutation, simple_query};

#[rustfmt::skip]
simple_query! {
	name = FetchLocalPlugins,
	ok = Vec<PluginInfo>,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let config = PluginManager::open_config(&back_state.paths)
				.context("Failed to open plugin config")?;
			let plugins = PluginManager::get_available_plugins(&back_state.paths)
				.context("Failed to get available plugins")?;

			let plugins = plugins.into_iter().map(|x| {
				let id = x.0;
				let manifest =
					PluginManager::read_plugin_manifest(&id, &back_state.paths).unwrap_or_default();

				PluginInfo {
					enabled: config.plugins.contains(&id),
					id,
					version: manifest.version,
					meta: manifest.meta,
					is_official: false,
				}
			});

			Ok(plugins.collect())
		})
	}
}

#[rustfmt::skip]
simple_query! {
	name = FetchRemotePlugins,
	ok = Vec<PluginInfo>,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(async move {
			let mut o = back_state.output();
			o.set_task(Task::FetchRemotePlugins);

			let verified_plugins = get_verified_plugins(&back_state.client, false)
				.await
				.context("Failed to get verified plugins")?;

			let verified_plugins = verified_plugins.into_values().map(|x| PluginInfo {
				id: x.id,
				meta: x.meta,
				version: x.version,
				enabled: false,
				is_official: x.github_owner == "Nitrolaunch",
			});

			Ok(verified_plugins
				.sorted_by_cached_key(|x| x.id.clone())
				.collect())
		})
	}
}

#[derive(Clone)]
pub struct PluginInfo {
	pub id: String,
	pub version: Option<String>,
	pub meta: PluginMetadata,
	pub enabled: bool,
	pub is_official: bool,
}

#[rustfmt::skip]
simple_query!(
	name = FetchPluginVersions,
	ok = Vec<String>,
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move {
			let mut o = back_state.output();
			o.set_task(Task::FetchPluginVersions);

			let verified_plugins = get_verified_plugins(&back_state.client, false)
				.await
				.context("Failed to get verified plugins")?;
			let plugin = verified_plugins.get(&id).context("Plugin does not exist")?;

			let assets = plugin
				.get_candidate_assets(None, &back_state.client)
				.await
				.context("Failed to get candidate assets")?;

			Ok(assets.into_iter().map(|x| x.version).unique().collect())
		})
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = InstallPlugin,
	ok = (),
	err = anyhow::Error,
	keys = (String, Option<String>),
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let (id, version) = keys.clone();

		query_spawn(async move {
			let mut o = back_state.output();
			o.set_task(Task::InstallPlugin);

			let verified_plugins = get_verified_plugins(&back_state.client, false)
				.await
				.context("Failed to get verified plugins")?;
			let plugin = verified_plugins.get(&id).context("Plugin does not exist")?;
			plugin.install(version.as_deref(), &back_state.paths, &back_state.client, &mut o)
				.await
				.context("Failed to install plugin")?;

			Ok(())
		})
	}

	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()>
	{
		QueriesStorage::<FetchLocalPlugins>::invalidate_all()
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = UninstallPlugin,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move { PluginManager::uninstall_plugin(&id, &back_state.paths) })
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		QueriesStorage::<FetchLocalPlugins>::invalidate_all()
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = EnablePlugin,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move { PluginManager::enable_plugin(&id, &back_state.paths) })
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		QueriesStorage::<FetchLocalPlugins>::invalidate_all()
	}
);

#[rustfmt::skip]
simple_mutation!(
	name = DisablePlugin,
	ok = (),
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let id = keys.clone();

		query_spawn(async move { PluginManager::disable_plugin(&id, &back_state.paths) })
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		QueriesStorage::<FetchLocalPlugins>::invalidate_all()
	}
);
