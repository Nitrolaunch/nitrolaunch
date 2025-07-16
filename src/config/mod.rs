/// Easy programatic creation of config
#[cfg(feature = "builder")]
pub mod builder;
/// Configuring instances
pub mod instance;
/// Configuring profile modifications
pub mod modifications;
/// Configuring packages
pub mod package;
/// Configuring plugins
pub mod plugin;
/// Configuring global preferences
pub mod preferences;
/// Configuring profiles
pub mod profile;

use self::instance::read_instance_config;
use crate::plugin::PluginManager;
use anyhow::{bail, Context};
use mcvm_config::profile::ProfileConfig;
use mcvm_config::ConfigDeser;
use mcvm_core::auth_crate::mc::ClientId;
use mcvm_core::io::{json_from_file, json_to_file_pretty};
use mcvm_core::user::UserManager;
use mcvm_plugin::hooks::{AddInstances, AddInstancesArg, AddProfiles, AddSupportedLoaders};
use mcvm_shared::id::{InstanceID, ProfileID};
use mcvm_shared::output::{MCVMOutput, MessageContents, MessageLevel};
use mcvm_shared::translate;
use mcvm_shared::util::is_valid_identifier;
use preferences::ConfigPreferences;
use profile::consolidate_profile_configs;

use super::instance::Instance;
use crate::io::paths::Paths;
use crate::pkg::reg::PkgRegistry;

use serde_json::json;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The data resulting from reading configuration.
/// Represents all of the configured data that MCVM will use
pub struct Config {
	/// The user manager
	pub users: UserManager,
	/// Instances
	pub instances: HashMap<InstanceID, Instance>,
	/// Profiles
	pub profiles: HashMap<ProfileID, ProfileConfig>,
	/// Consolidated profiles
	pub consolidated_profiles: HashMap<ProfileID, ProfileConfig>,
	/// The globally applied profile
	pub global_profile: ProfileConfig,
	/// Named groups of instances
	pub instance_groups: HashMap<Arc<str>, Vec<InstanceID>>,
	/// The registry of packages. Will include packages that are configured when created this way
	pub packages: PkgRegistry,
	/// Configured plugins
	pub plugins: PluginManager,
	/// Global user preferences
	pub prefs: ConfigPreferences,
}

impl Config {
	/// Get the config path
	pub fn get_path(paths: &Paths) -> PathBuf {
		paths.project.config_dir().join("mcvm.json")
	}

	/// Open the config from a file
	pub fn open(path: &Path) -> anyhow::Result<ConfigDeser> {
		if path.exists() {
			Ok(json_from_file(path).context("Failed to open config")?)
		} else {
			let config = default_config();
			json_to_file_pretty(path, &config).context("Failed to write default configuration")?;
			Ok(serde_json::from_value(config).context("Failed to parse default configuration")?)
		}
	}

	/// Create the default config at the specified path if it does not exist
	pub fn create_default(path: &Path) -> anyhow::Result<()> {
		if !path.exists() {
			let doc = default_config();
			json_to_file_pretty(path, &doc).context("Failed to write default configuration")?;
		}
		Ok(())
	}

	/// Create the Config struct from deserialized config
	async fn load_from_deser(
		mut config: ConfigDeser,
		plugins: PluginManager,
		show_warnings: bool,
		paths: &Paths,
		client_id: ClientId,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<Self> {
		let mut users = UserManager::new(client_id);
		let mut instances = HashMap::with_capacity(config.instances.len());
		// Preferences
		let (prefs, repositories) =
			ConfigPreferences::read(&config.preferences, &plugins, paths, o)
				.await
				.context("Failed to read preferences")?;

		let packages = PkgRegistry::new(repositories, &plugins);

		// Users
		for (user_id, user_config) in config.users.iter() {
			if !is_valid_identifier(user_id) {
				bail!("Invalid user ID '{user_id}'");
			}
			let user = user_config.to_user(user_id);
			// Disabled until we can verify game ownership.
			// We don't want to be a cracked launcher.
			if user.is_demo() {
				bail!("Unverified and Demo users are currently disabled");
			}

			users.add_user(user);
		}

		if let Some(default_user_id) = &config.default_user {
			if users.user_exists(default_user_id) {
				users
					.choose_user(default_user_id)
					.expect("Default user should exist");
			} else {
				bail!("Provided default user '{default_user_id}' does not exist");
			}
		} else if config.users.is_empty() && show_warnings {
			o.display(
				MessageContents::Warning(translate!(o, NoDefaultUser)),
				MessageLevel::Important,
			);
		} else if show_warnings {
			o.display(
				MessageContents::Warning(translate!(o, NoUsers)),
				MessageLevel::Important,
			);
		}

		// Add instances from plugins
		let arg = AddInstancesArg {};
		let results = plugins
			.call_hook(AddInstances, &arg, paths, o)
			.await
			.context("Failed to call add instances hook")?;
		for result in results {
			let result = result.result(o).await?;
			config.instances.extend(result);
		}
		// Add profiles from plugins
		let results = plugins
			.call_hook(AddProfiles, &arg, paths, o)
			.await
			.context("Failed to call add profiles hook")?;
		for result in results {
			let result = result.result(o).await?;
			config.profiles.extend(result);
		}

		// Consolidate profiles
		let consolidated_profiles =
			consolidate_profile_configs(config.profiles.clone(), config.global_profile.as_ref())
				.context("Failed to merge profiles")?;

		// Load extra supported loaders
		let mut supported_loaders = Vec::new();
		let results = plugins
			.call_hook(AddSupportedLoaders, &(), paths, o)
			.await
			.context("Failed to get supported loaders")?;
		for result in results {
			let result = result.result(o).await?;
			supported_loaders.extend(result);
		}

		// Instances
		for (instance_id, instance_config) in config.instances {
			let result = read_instance_config(
				instance_id.clone(),
				instance_config,
				&consolidated_profiles,
				&plugins,
				paths,
				o,
			)
			.await;

			let instance = match result {
				Ok(instance) => instance,
				Err(e) => {
					o.display(
						MessageContents::Error(translate!(
							o,
							InvalidInstanceConfig,
							"instance" = &instance_id,
							"error" = &format!("{e:#?}")
						)),
						MessageLevel::Important,
					);
					continue;
				}
			};

			if show_warnings
				&& !mcvm_config::instance::can_install_loader(&instance.config.loader)
				&& !supported_loaders.contains(&instance.config.loader)
			{
				o.display(
					MessageContents::Warning(translate!(
						o,
						ModificationNotSupported,
						"mod" = &format!("{}", instance.config.loader)
					)),
					MessageLevel::Important,
				);
			}

			instances.insert(instance_id, instance);
		}

		for group in config.instance_groups.keys() {
			if !is_valid_identifier(group) {
				bail!("Invalid ID for group '{group}'");
			}
		}

		Ok(Self {
			users,
			instances,
			profiles: config.profiles,
			consolidated_profiles,
			global_profile: config.global_profile.unwrap_or_default(),
			instance_groups: config.instance_groups,
			packages,
			plugins,
			prefs,
		})
	}

	/// Load the configuration from the config file
	pub async fn load(
		path: &Path,
		plugins: PluginManager,
		show_warnings: bool,
		paths: &Paths,
		client_id: ClientId,
		o: &mut impl MCVMOutput,
	) -> anyhow::Result<Self> {
		let obj = Self::open(path)?;
		Self::load_from_deser(obj, plugins, show_warnings, paths, client_id, o).await
	}
}

/// Default program configuration
fn default_config() -> serde_json::Value {
	json!(
		{
			"users": {
				"example": {
					"type": "microsoft"
				}
			},
			"default_user": "example",
			"profiles": {
				"1.20": {
					"version": "1.19.3",
					"loader": "vanilla",
					"server_type": "none"
				}
			},
			"instances": {
				"example-client": {
					"from": "1.20",
					"type": "client"
				},
				"example-server": {
					"from": "1.20",
					"type": "server"
				}
			}
		}
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	use mcvm_shared::output;

	#[tokio::test]
	async fn test_default_config() {
		let deser = serde_json::from_value(default_config()).unwrap();
		Config::load_from_deser(
			deser,
			PluginManager::new(),
			true,
			&Paths::new_no_create().unwrap(),
			ClientId::new(String::new()),
			&mut output::Simple(output::MessageLevel::Debug),
		)
		.await
		.unwrap();
	}
}
