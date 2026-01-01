use anyhow::{anyhow, bail, Context};
use nitro_config::instance::InstanceConfig;
use nitro_config::template::TemplateConfig;
use nitro_config::ConfigDeser;
use nitro_config::{package::PackageConfigDeser, user::UserConfig};
use nitro_core::io::json_to_file_pretty;
use nitro_plugin::hook::hooks::{
	SaveInstanceConfig, SaveInstanceConfigArg, SaveTemplateConfig, SaveTemplateConfigArg,
};
use nitro_shared::output::NoOp;

use crate::io::paths::Paths;
use crate::plugin::PluginManager;
use nitro_shared::id::{InstanceID, TemplateID};

use super::Config;

/// A modification operation that can be applied to the config
pub enum ConfigModification {
	/// Adds a new user
	AddUser(String, UserConfig),
	/// Adds or updates an instance
	AddInstance(InstanceID, InstanceConfig),
	/// Adds or updates a template
	AddTemplate(TemplateID, TemplateConfig),
	/// Adds a new package to an instance
	AddPackage(InstanceID, PackageConfigDeser),
	/// Removes a user
	RemoveUser(String),
	/// Removes an instance
	RemoveInstance(InstanceID),
	/// Removes a template
	RemoveTemplate(InstanceID),
}

/// Applies modifications to the config
pub async fn apply_modifications(
	config: &mut ConfigDeser,
	modifications: Vec<ConfigModification>,
	paths: &Paths,
	plugins: &PluginManager,
) -> anyhow::Result<()> {
	for modification in modifications {
		match modification {
			ConfigModification::AddUser(id, user) => {
				config.users.insert(id, user);
			}
			ConfigModification::AddInstance(instance_id, instance) => {
				if let Some(plugin) = &instance.source_plugin {
					if !instance.is_editable {
						bail!("Plugin instance is not editable");
					}

					let result = plugins
						.call_hook_on_plugin(
							SaveInstanceConfig,
							&plugin.clone(),
							&SaveInstanceConfigArg {
								id: instance_id.to_string(),
								config: instance,
							},
							paths,
							&mut NoOp,
						)
						.await?;
					if let Some(result) = result {
						result.result(&mut NoOp).await?;
					}
				} else {
					config.instances.insert(instance_id, instance);
				}
			}
			ConfigModification::AddTemplate(template_id, template) => {
				if let Some(plugin) = &template.instance.source_plugin {
					if !template.instance.is_editable {
						bail!("Plugin template is not editable");
					}

					let result = plugins
						.call_hook_on_plugin(
							SaveTemplateConfig,
							&plugin.clone(),
							&SaveTemplateConfigArg {
								id: template_id.to_string(),
								config: template,
							},
							paths,
							&mut NoOp,
						)
						.await?;
					if let Some(result) = result {
						result.result(&mut NoOp).await?;
					}
				} else {
					config.templates.insert(template_id, template);
				}
			}
			ConfigModification::AddPackage(instance_id, package) => {
				let instance = config
					.instances
					.get_mut(&instance_id)
					.ok_or(anyhow!("Unknown instance '{instance_id}'"))?;
				instance.packages.push(package);
			}
			ConfigModification::RemoveUser(user) => {
				config.users.remove(&user);
			}
			ConfigModification::RemoveInstance(instance) => {
				config.instances.remove(&instance);
			}
			ConfigModification::RemoveTemplate(template) => {
				config.templates.remove(&template);
			}
		};
	}
	Ok(())
}

/// Applies modifications to the config and writes it to the config file
pub async fn apply_modifications_and_write(
	config: &mut ConfigDeser,
	modifications: Vec<ConfigModification>,
	paths: &Paths,
	plugins: &PluginManager,
) -> anyhow::Result<()> {
	apply_modifications(config, modifications, paths, plugins).await?;
	let path = Config::get_path(paths);
	// Backup the contents first
	std::fs::copy(&path, paths.config.join("nitro_write_backup.json"))
		.context("Failed to backup config")?;
	json_to_file_pretty(path, config).context("Failed to write modified configuration")?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use nitro_config::user::UserVariant;

	use super::*;

	#[tokio::test]
	async fn test_user_add_modification() {
		let mut config = ConfigDeser::default();

		let user_config = UserConfig::Simple(UserVariant::Demo {});

		let modifications = vec![ConfigModification::AddUser("bob".into(), user_config)];

		let paths = Paths::new_no_create().unwrap();
		let plugins = PluginManager::new(&paths);

		apply_modifications(&mut config, modifications, &paths, &plugins)
			.await
			.unwrap();
		assert!(config.users.contains_key("bob"));
	}
}
