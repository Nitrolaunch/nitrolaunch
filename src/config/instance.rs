use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::package::read_package_config;
use crate::instance::launch::LaunchOptions;
use crate::instance::{InstKind, Instance, InstanceStoredConfig};
use crate::io::paths::Paths;
use anyhow::{bail, ensure, Context};
use nitro_config::instance::{is_valid_instance_id, InstanceConfig, LaunchConfig, LaunchMemory};
use nitro_config::package::PackageConfigDeser;
use nitro_config::profile::{ProfileConfig, ProfileLoaderConfiguration};
use nitro_core::io::java::args::MemoryNum;
use nitro_core::io::java::install::JavaInstallationKind;
use nitro_pkg::{PkgRequest, PkgRequestSource};
use nitro_plugin::hooks::{ModifyInstanceConfig, ModifyInstanceConfigArgument};
use nitro_shared::id::{InstanceID, ProfileID};
use nitro_shared::loaders::Loader;
use nitro_shared::output::NitroOutput;
use nitro_shared::versions::parse_versioned_string;
use nitro_shared::Side;

use crate::plugin::PluginManager;

/// Read the config for an instance to create the instance
pub async fn read_instance_config(
	id: InstanceID,
	mut config: InstanceConfig,
	profiles: &HashMap<ProfileID, ProfileConfig>,
	plugins: &PluginManager,
	paths: &Paths,
	o: &mut impl NitroOutput,
) -> anyhow::Result<Instance> {
	if !is_valid_instance_id(&id) {
		bail!("Invalid instance ID '{}'", id.to_string());
	}

	// Get the parent profile if it is specified
	let profiles: anyhow::Result<Vec<_>> = config
		.common
		.from
		.iter()
		.map(|x| {
			profiles
				.get(&ProfileID::from(x.clone()))
				.with_context(|| format!("Derived profile '{x}' does not exist"))
		})
		.collect();
	let profiles = profiles?;

	let original_config = config.clone();

	// Merge with the profile
	for profile in &profiles {
		let mut profile_config = profile.instance.clone();
		profile_config.merge(config);
		config = profile_config;
	}

	let mut original_config_with_profiles = config.clone();

	let side = config.side.context("Instance type was not specified")?;

	// Consolidate all of the package configs into the instance package config list
	let packages = consolidate_package_configs(&profiles, &config, side);

	original_config_with_profiles.common.packages = packages.clone();

	let read_packages = packages
		.clone()
		.into_iter()
		.map(|x| read_package_config(x, config.common.package_stability.unwrap_or_default()))
		.collect();

	// Loader
	let profile_loaders =
		profiles
			.iter()
			.fold(ProfileLoaderConfiguration::default(), |mut acc, profile| {
				acc.merge(&profile.loader);
				acc
			});

	let loader = match side {
		Side::Client => config
			.common
			.loader
			.clone()
			.or(profile_loaders.client().cloned()),
		Side::Server => config
			.common
			.loader
			.clone()
			.or(profile_loaders.server().cloned()),
	};

	original_config_with_profiles.common.loader = loader.clone();

	// Apply plugins
	let arg = ModifyInstanceConfigArgument {
		config: config.clone(),
	};
	let results = plugins
		.call_hook(ModifyInstanceConfig, &arg, paths, o)
		.await
		.context("Failed to apply plugin instance modifications")?;
	for result in results {
		let result = result.result(o).await?;
		config.merge(result.config);
	}

	let mut original_config_with_profiles_and_plugins = config.clone();
	original_config_with_profiles_and_plugins.common.loader = loader.clone();
	original_config_with_profiles_and_plugins.common.packages = packages.clone();

	let kind = match side {
		Side::Client => InstKind::client(config.window),
		Side::Server => InstKind::server(),
	};

	let (loader, loader_version) = if let Some(loader) = loader {
		let (loader, version) = parse_versioned_string(&loader);
		(Loader::parse_from_str(loader), Some(version))
	} else {
		(Loader::Vanilla, None)
	};

	let version = config
		.common
		.version
		.clone()
		.context("Instance is missing a Minecraft version")?
		.to_mc_version();

	let stored_config = InstanceStoredConfig {
		name: config.name,
		icon: config.icon,
		version,
		loader,
		loader_version,
		launch: launch_config_to_options(config.common.launch)?,
		datapack_folder: config.common.datapack_folder,
		packages: read_packages,
		package_stability: config.common.package_stability.unwrap_or_default(),
		package_overrides: config.common.overrides,
		game_dir: config.common.game_dir.map(PathBuf::from),
		original_config,
		original_config_with_profiles,
		original_config_with_profiles_and_plugins,
		plugin_config: config.common.plugin_config,
	};

	let instance = Instance::new(kind, id, stored_config);

	Ok(instance)
}

/// Parse and finalize this LaunchConfig into LaunchOptions
pub fn launch_config_to_options(config: LaunchConfig) -> anyhow::Result<LaunchOptions> {
	let min_mem = match &config.memory {
		LaunchMemory::None => None,
		LaunchMemory::Single(string) => MemoryNum::parse(string),
		LaunchMemory::Both { min, .. } => MemoryNum::parse(min),
	};
	let max_mem = match &config.memory {
		LaunchMemory::None => None,
		LaunchMemory::Single(string) => MemoryNum::parse(string),
		LaunchMemory::Both { max, .. } => MemoryNum::parse(max),
	};
	if let Some(min_mem) = &min_mem {
		if let Some(max_mem) = &max_mem {
			ensure!(
				min_mem.to_bytes() <= max_mem.to_bytes(),
				"Minimum memory must be less than or equal to maximum memory"
			);
		}
	}
	Ok(LaunchOptions {
		jvm_args: config.args.jvm.parse(),
		game_args: config.args.game.parse(),
		min_mem,
		max_mem,
		java: JavaInstallationKind::parse(&config.java),
		env: config.env,
		wrapper: config.wrapper,
		quick_play: config.quick_play,
		use_log4j_config: config.use_log4j_config,
	})
}

/// Combines all of the package configs from global, profile, and instance together into
/// the configurations for just one instance
pub fn consolidate_package_configs(
	profiles: &[&ProfileConfig],
	instance: &InstanceConfig,
	side: Side,
) -> Vec<PackageConfigDeser> {
	// We use a map so that we can override packages from more general sources
	// with those from more specific ones
	let mut map = HashMap::new();
	for profile in profiles {
		for pkg in profile.packages.iter_global() {
			// let pkg = read_package_config(pkg.clone(), stability, PackageConfigSource::Instance);
			map.insert(
				PkgRequest::parse(pkg.get_pkg_id(), PkgRequestSource::UserRequire).id,
				pkg.clone(),
			);
		}
		for pkg in profile.packages.iter_side(side) {
			// let pkg = read_package_config(pkg.clone(), stability, PackageConfigSource::Profile);
			map.insert(
				PkgRequest::parse(pkg.get_pkg_id(), PkgRequestSource::UserRequire).id,
				pkg.clone(),
			);
		}
	}
	for pkg in &instance.common.packages {
		// let pkg = read_package_config(pkg.clone(), stability, PackageConfigSource::Instance);
		map.insert(
			PkgRequest::parse(pkg.get_pkg_id(), PkgRequestSource::UserRequire).id,
			pkg.clone(),
		);
	}

	map.into_values().collect()
}

#[cfg(test)]
mod tests {
	use nitro_config::instance::QuickPlay;
	use serde::Deserialize;

	#[test]
	fn test_quickplay_deser() {
		#[derive(Deserialize)]
		struct Test {
			quick_play: QuickPlay,
		}

		let test = serde_json::from_str::<Test>(
			r#"{
			"quick_play": {
				"type": "server",
				"server": "localhost",
				"port": 25565,
				"world": "test",
				"realm": "my_realm"
			}	
		}"#,
		)
		.unwrap();
		assert_eq!(
			test.quick_play,
			QuickPlay::Server {
				server: "localhost".into(),
				port: Some(25565)
			}
		);
	}
}
