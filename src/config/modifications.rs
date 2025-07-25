use anyhow::{anyhow, Context};
use nitro_config::instance::InstanceConfig;
use nitro_config::profile::ProfileConfig;
use nitro_config::ConfigDeser;
use nitro_config::{package::PackageConfigDeser, user::UserConfig};
use nitro_core::io::json_to_file_pretty;

use crate::io::paths::Paths;
use nitro_shared::id::{InstanceID, ProfileID};

use super::Config;

/// A modification operation that can be applied to the config
pub enum ConfigModification {
	/// Adds a new user
	AddUser(String, UserConfig),
	/// Adds a new profile
	AddProfile(ProfileID, ProfileConfig),
	/// Adds a new instance
	AddInstance(InstanceID, InstanceConfig),
	/// Adds a new package to an instance
	AddPackage(InstanceID, PackageConfigDeser),
	/// Removes a user
	RemoveUser(String),
	/// Removes an instance
	RemoveInstance(InstanceID),
	/// Removes a profile
	RemoveProfile(InstanceID),
}

/// Applies modifications to the config
pub fn apply_modifications(
	config: &mut ConfigDeser,
	modifications: Vec<ConfigModification>,
) -> anyhow::Result<()> {
	for modification in modifications {
		match modification {
			ConfigModification::AddUser(id, user) => {
				config.users.insert(id, user);
			}
			ConfigModification::AddProfile(id, profile) => {
				config.profiles.insert(id, profile);
			}
			ConfigModification::AddInstance(instance_id, instance) => {
				config.instances.insert(instance_id, instance);
			}
			ConfigModification::AddPackage(instance_id, package) => {
				let instance = config
					.instances
					.get_mut(&instance_id)
					.ok_or(anyhow!("Unknown instance '{instance_id}'"))?;
				instance.common.packages.push(package);
			}
			ConfigModification::RemoveUser(user) => {
				config.users.remove(&user);
			}
			ConfigModification::RemoveInstance(instance) => {
				config.instances.remove(&instance);
			}
			ConfigModification::RemoveProfile(profile) => {
				config.profiles.remove(&profile);
			}
		};
	}
	Ok(())
}

/// Applies modifications to the config and writes it to the config file
pub fn apply_modifications_and_write(
	config: &mut ConfigDeser,
	modifications: Vec<ConfigModification>,
	paths: &Paths,
) -> anyhow::Result<()> {
	apply_modifications(config, modifications)?;
	let path = Config::get_path(paths);
	// Backup the contents first
	std::fs::copy(
		&path,
		paths.project.config_dir().join("nitro_write_backup.json"),
	)
	.context("Failed to backup config")?;
	json_to_file_pretty(path, config).context("Failed to write modified configuration")?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use nitro_config::user::UserVariant;

	use super::*;

	#[test]
	fn test_user_add_modification() {
		let mut config = ConfigDeser::default();

		let user_config = UserConfig::Simple(UserVariant::Demo {});

		let modifications = vec![ConfigModification::AddUser("bob".into(), user_config)];

		apply_modifications(&mut config, modifications).unwrap();
		assert!(config.users.contains_key("bob"));
	}
}
