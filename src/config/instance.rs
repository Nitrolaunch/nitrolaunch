use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::package::read_package_config;
use crate::instance::launch::LaunchOptions;
use crate::instance::{InstKind, Instance, InstanceStoredConfig};
use crate::io::paths::Paths;
use anyhow::{bail, ensure, Context};
use nitro_config::instance::{is_valid_instance_id, InstanceConfig, LaunchConfig, LaunchMemory};
use nitro_config::template::TemplateConfig;
use nitro_core::io::java::install::JavaInstallationKind;
use nitro_core::util::versions::MinecraftVersion;
use nitro_plugin::hook::hooks::{ModifyInstanceConfig, ModifyInstanceConfigArgument};
use nitro_shared::id::{InstanceID, TemplateID};
use nitro_shared::java_args::MemoryNum;
use nitro_shared::loaders::Loader;
use nitro_shared::output::NitroOutput;
use nitro_shared::versions::parse_versioned_string;
use nitro_shared::Side;

use crate::plugin::PluginManager;

/// Read the config for an instance to create the instance
pub async fn read_instance_config(
	id: InstanceID,
	mut config: InstanceConfig,
	templates: &HashMap<TemplateID, TemplateConfig>,
	plugins: &PluginManager,
	paths: &Paths,
	o: &mut impl NitroOutput,
) -> anyhow::Result<Instance> {
	if !is_valid_instance_id(&id) {
		bail!("Invalid instance ID '{}'", id);
	}

	let original_config = config.clone();
	let mut config = config.apply_templates(templates)?;
	let original_config_with_templates = config.clone();

	// Apply plugins
	let arg = ModifyInstanceConfigArgument {
		config: config.clone(),
	};
	let mut results = plugins
		.call_hook(ModifyInstanceConfig, &arg, paths, o)
		.await
		.context("Failed to apply plugin instance modifications")?;
	while let Some(result) = results.next_result(o).await? {
		config.merge(result.config);
	}

	let original_config_with_templates_and_plugins = config.clone();

	let kind = match config.side.unwrap() {
		Side::Client => InstKind::client(config.window),
		Side::Server => InstKind::server(),
	};

	let (loader, loader_version) = if let Some(loader) = &config.loader {
		let (loader, version) = parse_versioned_string(loader);
		(Loader::parse_from_str(loader), Some(version))
	} else {
		(Loader::Vanilla, None)
	};

	let version = MinecraftVersion::from_deser(
		&config
			.version
			.clone()
			.context("Instance is missing a Minecraft version")?,
	);

	let read_packages = config
		.packages
		.clone()
		.into_iter()
		.map(|x| read_package_config(x, config.package_stability.unwrap_or_default()))
		.collect();

	let stored_config = InstanceStoredConfig {
		name: config.name,
		icon: config.icon,
		version,
		loader,
		loader_version,
		launch: launch_config_to_options(config.launch)?,
		datapack_folder: config.datapack_folder,
		packages: read_packages,
		package_stability: config.package_stability.unwrap_or_default(),
		package_overrides: config.overrides,
		inst_dir_override: config.dir.map(PathBuf::from),
		custom_launch: config.custom_launch,
		original_config,
		original_config_with_templates,
		original_config_with_templates_and_plugins,
		plugin_config: config.plugin_config,
	};

	let instance = Instance::new(kind, id, stored_config, paths);

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
		java: JavaInstallationKind::parse(config.java.as_deref().unwrap_or("auto")),
		env: config.env,
		wrapper: config.wrapper,
		quick_play: config.quick_play,
		use_log4j_config: config.use_log4j_config,
	})
}
