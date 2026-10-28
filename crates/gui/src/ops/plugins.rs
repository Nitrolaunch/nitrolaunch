use anyhow::Context;
use itertools::Itertools;
use nitrolaunch::{
	config::plugin::PluginsConfig,
	config_crate::ConfigKind,
	core::io::{json_from_file, json_to_file_pretty},
	plugin::{PluginManager, install::get_verified_plugins},
	plugin_crate::plugin::PluginMetadata,
	shared::output::{MessageContents, NitroOutput},
};

use crate::{
	components::input::control::ControlledConfig, ops::task::Task, prelude::*, simple_mutation,
	simple_query,
};

#[rustfmt::skip]
simple_query! {
	name = FetchLocalPlugins,
	ok = Vec<PluginInfo>,
	err = anyhow::Error,
	keys = (),
	fn run(&self, _: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		query_spawn(back_state.0.clone(), async move {
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

		query_spawn(back_state.0.clone(), async move {
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

		query_spawn(back_state.0.clone(), async move {
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

		let task = async move {
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
		};

		self.back_state
			.register_task(Task::InstallPlugin, tokio::spawn(task));
		async { Ok(()) }
	}

	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()>
	{
		self.back_state.invalidate_plugins()
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

		query_spawn(back_state.0.clone(), async move { PluginManager::uninstall_plugin(&id, &back_state.paths) })
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		self.back_state.invalidate_plugins()
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

		query_spawn(back_state.0.clone(), async move { PluginManager::enable_plugin(&id, &back_state.paths) })
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		self.back_state.invalidate_plugins()
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

		query_spawn(back_state.0.clone(), async move { PluginManager::disable_plugin(&id, &back_state.paths) })
	}
	fn on_settled(
		&self,
		_keys: &Self::Keys,
		_result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		self.back_state.invalidate_plugins()
	}
);

simple_query!(
	name = FetchConfigCreationPlugins,
	ok = Vec<PluginInfo>,
	err = anyhow::Error,
	keys = ConfigKind,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let ty = *keys;

		query_spawn(back_state.0.clone(), async move {
			let out = back_state
				.plugins
				.get_lock()
				.await
				.manager
				.iter_plugins()
				.filter(|x| if ty == ConfigKind::Instance { x.manifest.supports_instance_creation } else { x.manifest.supports_template_creation })
				.map(|x| PluginInfo {
					id: x.get_id().clone(),
					version: x.manifest.version.clone(),
					meta: x.manifest.meta.clone(),
					enabled: false,
					is_official: false,
				}).collect();

			Ok(out)
		})
	}
);

simple_mutation!(
	name = InstallDefaultPlugins,
	ok = (),
	err = anyhow::Error,
	keys = (),
	fn run(&self, _keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();

		let default_plugins = [
			"fabric_quilt",
			"forge",
			"modrinth",
			"smithed",
			"curseforge",
			"multimc_transfer",
			"xmcl_transfer",
			"mojang_transfer",
			"themes",
		];

		let task = async move {
			let mut o = back_state.output();
			o.set_task(Task::InstallDefaultPlugins);

			let verified_list = get_verified_plugins(&back_state.client, false)
				.await
				.context("Failed to get verified plugin list")?;

			for (i, plugin) in default_plugins.into_iter().enumerate() {
				let Some(plugin) = verified_list.get(plugin) else {
					back_state.log(format!("Unknown plugin '{plugin}'"));
					continue;
				};

				if let Err(e) = plugin
					.install(None, &back_state.paths, &back_state.client, &mut o)
					.await
				{
					back_state.log(format!("Failed to install plugin '{}': {e:?}", plugin.id));
				}

				o.display(MessageContents::Progress {
					current: i as u32,
					total: default_plugins.len() as u32,
				});
			}

			back_state.invalidate_plugins().await;

			Ok::<_, anyhow::Error>(())
		};

		self.back_state
			.register_task(Task::InstallDefaultPlugins, tokio::spawn(task));

		async { Ok(()) }
	}
);

simple_query!(
	name = FetchPluginConfig,
	ok = ControlledConfig,
	err = anyhow::Error,
	keys = String,
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let plugin_id = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			back_state.reload_plugins().await;
			let config = back_state
				.plugins
				.get_lock()
				.await
				.configs
				.iter()
				.find(|x| x.id == plugin_id)
				.and_then(|x| x.custom_config.clone())
				.unwrap_or(serde_json::Value::Object(Default::default()));
			if let serde_json::Value::Object(object) = config {
				Ok(ControlledConfig::new(object))
			} else {
				Ok(ControlledConfig::new(serde_json::Map::new()))
			}
		})
	}
);

simple_mutation!(
	name = SavePluginConfig,
	ok = (),
	err = anyhow::Error,
	keys = (String, serde_json::Map<String, serde_json::Value>),
	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let back_state = self.back_state.clone();
		let (plugin_id, plugin_config) = keys.clone();

		query_spawn(back_state.0.clone(), async move {
			let config_path = PluginManager::get_config_path(&back_state.paths);
			let mut config: PluginsConfig = json_from_file(&config_path)?;
			config.config.insert(
				plugin_id,
				serde_json::Value::Object(plugin_config),
			);
			json_to_file_pretty(config_path, &config)?;
			Ok::<_, anyhow::Error>(())
		})
	}
);
