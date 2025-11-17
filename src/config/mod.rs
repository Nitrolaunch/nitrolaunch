/// Configuring instances
pub mod instance;
/// Configuring instance modifications
pub mod modifications;
/// Configuring packages
pub mod package;
/// Configuring plugins
pub mod plugin;
/// Configuring global preferences
pub mod preferences;
/// Configuring templates
pub mod template;

use self::instance::read_instance_config;
use crate::plugin::PluginManager;
use anyhow::Context;
use nitro_config::template::TemplateConfig;
use nitro_config::ConfigDeser;
use nitro_core::auth_crate::mc::ClientId;
use nitro_core::io::{json_from_file, json_to_file_pretty};
use nitro_core::user::UserManager;
use nitro_plugin::hook::hooks::{AddInstances, AddInstancesArg, AddSupportedLoaders, AddTemplates};
use nitro_shared::id::{InstanceID, TemplateID};
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::util::is_valid_identifier;
use nitro_shared::{skip_fail, translate};
use preferences::ConfigPreferences;
use template::consolidate_template_configs;
use version_compare::Version;

use super::instance::Instance;
use crate::io::paths::Paths;
use crate::pkg::reg::PkgRegistry;

use serde_json::json;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The data resulting from reading configuration.
/// Represents all of the configured data that Nitrolaunch will use
pub struct Config {
	/// The user manager
	pub users: UserManager,
	/// Instances
	pub instances: HashMap<InstanceID, Instance>,
	/// templates
	pub templates: HashMap<TemplateID, TemplateConfig>,
	/// Consolidated templates
	pub consolidated_templates: HashMap<TemplateID, TemplateConfig>,
	/// The globally applied template
	pub base_template: TemplateConfig,
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
		paths.config.join("nitro.json")
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
		o: &mut impl NitroOutput,
	) -> Self {
		let _ = check_nitro_version(paths, o);

		let mut users = UserManager::new(client_id);
		let mut instances = HashMap::with_capacity(config.instances.len());
		// Preferences
		let (prefs, repositories) =
			ConfigPreferences::read(&config.preferences, &plugins, paths, o).await;

		let packages = PkgRegistry::new(repositories, &plugins);

		// Users
		for (user_id, user_config) in config.users.iter() {
			if !is_valid_identifier(user_id) {
				o.display(
					MessageContents::Error(format!("Invalid user ID '{user_id}'")),
					MessageLevel::Important,
				);
				continue;
			}
			let user = user_config.to_user(user_id);
			// Disabled until we can verify game ownership.
			// We don't want to be a cracked launcher.
			if user.is_demo() {
				o.display(
					MessageContents::Error(
						"Unverified and Demo users are currently disabled".into(),
					),
					MessageLevel::Important,
				);
			}

			users.add_user(user);
		}

		if let Some(default_user_id) = &config.default_user {
			if users.user_exists(default_user_id) {
				users
					.choose_user(default_user_id)
					.expect("Default user should exist");
			} else {
				o.display(
					MessageContents::Error(format!(
						"Provided default user '{default_user_id}' does not exist"
					)),
					MessageLevel::Important,
				);
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
		let results = plugins.call_hook(AddInstances, &arg, paths, o).await;

		match results {
			Ok(results) => {
				for result in results {
					let result = skip_fail!(result.result(o).await);
					for (id, mut instance) in result.into_iter() {
						if config.instances.contains_key(&id) {
							continue;
						}

						instance.from_plugin = true;
						config.instances.insert(id, instance);
					}
				}
			}
			Err(e) => {
				o.display(
					MessageContents::Error(format!("Failed to add instances from plugins: {e:?}")),
					MessageLevel::Important,
				);
			}
		}
		// Add templates from plugins
		let results = plugins.call_hook(AddTemplates, &arg, paths, o).await;
		match results {
			Ok(results) => {
				for result in results {
					let result = skip_fail!(result.result(o).await);
					for (id, mut template) in result.into_iter() {
						if config.templates.contains_key(&id) {
							continue;
						}

						template.instance.from_plugin = true;
						config.templates.insert(id, template);
					}
				}
			}
			Err(e) => {
				o.display(
					MessageContents::Error(format!("Failed to add templates from plugins: {e:?}")),
					MessageLevel::Important,
				);
			}
		}

		// Consolidate templates
		let consolidated_templates = consolidate_template_configs(
			config.templates.clone(),
			config.base_template.as_ref(),
			o,
		);

		// Load extra supported loaders
		let mut supported_loaders = Vec::new();
		let results = plugins.call_hook(AddSupportedLoaders, &(), paths, o).await;

		match results {
			Ok(results) => {
				for result in results {
					let result = skip_fail!(result.result(o).await);
					supported_loaders.extend(result);
				}
			}
			Err(e) => {
				o.display(
					MessageContents::Error(format!(
						"Failed to get supported loaders from plugins: {e:?}"
					)),
					MessageLevel::Important,
				);
			}
		}

		// Instances
		for (instance_id, instance_config) in config.instances {
			let result = read_instance_config(
				instance_id.clone(),
				instance_config,
				&consolidated_templates,
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
				&& !nitro_config::instance::can_install_loader(&instance.config.loader)
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
				o.display(
					MessageContents::Error(format!("Invalid ID for instance group '{group}'")),
					MessageLevel::Important,
				);
			}
		}

		Self {
			users,
			instances,
			templates: config.templates,
			consolidated_templates,
			base_template: config.base_template.unwrap_or_default(),
			instance_groups: config.instance_groups,
			packages,
			plugins,
			prefs,
		}
	}

	/// Load the configuration from the config file
	pub async fn load(
		path: &Path,
		plugins: PluginManager,
		show_warnings: bool,
		paths: &Paths,
		client_id: ClientId,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Self> {
		let obj = Self::open(path)?;
		Ok(Self::load_from_deser(obj, plugins, show_warnings, paths, client_id, o).await)
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
			"templates": {
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

/// Checks and updates the currently installed Nitro version and warns the user
pub fn check_nitro_version(paths: &Paths, o: &mut impl NitroOutput) -> anyhow::Result<()> {
	let path = paths.internal.join("nitro_version");

	if path.exists() {
		let contents = std::fs::read_to_string(&path)?;
		let contents = contents.trim_end();

		let current_version = Version::from(contents).context("Current version failed to parse")?;
		let new_version = Version::from(crate::VERSION).context("New version failed to parse")?;

		if current_version.compare_to(new_version, version_compare::Cmp::Gt) {
			o.display(
				MessageContents::Warning(translate!(
					o,
					WrongNitroVersion,
					"current" = &contents,
					"new" = crate::VERSION
				)),
				MessageLevel::Important,
			);
		} else {
			std::fs::write(path, crate::VERSION)?;
		}
	} else {
		std::fs::write(path, crate::VERSION)?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	use nitro_shared::output;

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
		.await;
	}
}
