use std::collections::HashMap;

use anyhow::bail;
use nitro_shared::id::TemplateID;
use nitro_shared::Side;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::instance::InstanceConfig;
use super::package::PackageConfigDeser;

/// Configuration for a template
#[derive(Deserialize, Serialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct TemplateConfig {
	/// The configuration for the instance
	#[serde(flatten)]
	pub instance: InstanceConfig,
	/// Loader configuration
	#[serde(default)]
	pub loader: TemplateLoaderConfiguration,
	/// Package configuration
	#[serde(default)]
	pub packages: TemplatePackageConfiguration,
}

impl TemplateConfig {
	/// Merge this template with another one
	pub fn merge(&mut self, other: Self) {
		self.instance.merge(other.instance);
		self.loader.merge(&other.loader);
		self.packages.merge(other.packages);
	}
}

/// Different representations of loader configuration on a template
#[derive(Deserialize, Serialize, Debug, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum TemplateLoaderConfiguration {
	/// Same loader for client and server
	Simple(Option<String>),
	/// Full configuration
	Full {
		/// Loader for the client
		client: Option<String>,
		/// Loader for the server
		server: Option<String>,
	},
}

impl Default for TemplateLoaderConfiguration {
	fn default() -> Self {
		Self::Simple(None)
	}
}

impl TemplateLoaderConfiguration {
	/// Gets the client side of this configuration
	pub fn client(&self) -> Option<&String> {
		match self {
			Self::Simple(loader) => loader.as_ref(),
			Self::Full { client, .. } => client.as_ref(),
		}
	}

	/// Gets the server side of this configuration
	pub fn server(&self) -> Option<&String> {
		match self {
			Self::Simple(loader) => loader.as_ref(),
			Self::Full { server, .. } => server.as_ref(),
		}
	}

	/// Merges this configuration with another one
	pub fn merge(&mut self, other: &Self) {
		let out = Self::Full {
			client: other.client().or(self.client()).cloned(),
			server: other.server().or(self.server()).cloned(),
		};
		*self = if out.client() == out.server() {
			Self::Simple(out.client().cloned())
		} else {
			out
		};
	}
}

/// Different representations of package configuration on a template
#[derive(Deserialize, Serialize, Debug, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum TemplatePackageConfiguration {
	/// Is just a list of packages for every instance
	Simple(Vec<PackageConfigDeser>),
	/// Full configuration
	Full {
		/// Packages to apply to every instance
		#[serde(default)]
		global: Vec<PackageConfigDeser>,
		/// Packages to apply to only clients
		#[serde(default)]
		client: Vec<PackageConfigDeser>,
		/// Packages to apply to only servers
		#[serde(default)]
		server: Vec<PackageConfigDeser>,
	},
}

impl Default for TemplatePackageConfiguration {
	fn default() -> Self {
		Self::Simple(Vec::new())
	}
}

impl TemplatePackageConfiguration {
	/// Merge this configuration with one from another template, with right taking precedence
	pub fn merge(&mut self, other: Self) {
		match (&mut *self, other) {
			(Self::Simple(left), Self::Simple(right)) => {
				left.extend(right);
			}
			(Self::Full { global, .. }, Self::Simple(right)) => {
				global.extend(right);
			}
			(
				Self::Simple(left),
				Self::Full {
					global,
					client,
					server,
				},
			) => {
				left.extend(global);
				*self = Self::Full {
					global: left.clone(),
					client,
					server,
				};
			}
			(
				Self::Full {
					global: global1,
					client: client1,
					server: server1,
				},
				Self::Full {
					global: global2,
					client: client2,
					server: server2,
				},
			) => {
				global1.extend(global2);
				client1.extend(client2);
				server1.extend(server2);
			}
		}
	}

	/// Validate all the configured packages
	pub fn validate(&self) -> anyhow::Result<()> {
		match &self {
			Self::Simple(global) => {
				for pkg in global {
					pkg.validate()?;
				}
			}
			Self::Full {
				global,
				client,
				server,
			} => {
				for pkg in global.iter().chain(client.iter()).chain(server.iter()) {
					pkg.validate()?;
				}
			}
		}

		Ok(())
	}

	/// Iterate over all of the packages
	pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a PackageConfigDeser> + 'a> {
		match &self {
			Self::Simple(global) => Box::new(global.iter()),
			Self::Full {
				global,
				client,
				server,
			} => Box::new(global.iter().chain(client.iter()).chain(server.iter())),
		}
	}

	/// Iterate over the global package list
	pub fn iter_global(&self) -> impl Iterator<Item = &PackageConfigDeser> {
		match &self {
			Self::Simple(global) => global,
			Self::Full { global, .. } => global,
		}
		.iter()
	}

	/// Iterate over the package list for a specific side
	pub fn iter_side(&self, side: Side) -> impl Iterator<Item = &PackageConfigDeser> {
		match &self {
			Self::Simple(..) => [].iter(),
			Self::Full { client, server, .. } => match side {
				Side::Client => client.iter(),
				Side::Server => server.iter(),
			},
		}
	}

	/// Adds a package to the global list
	pub fn add_global_package(&mut self, pkg: PackageConfigDeser) {
		match self {
			Self::Simple(global) => global.push(pkg),
			Self::Full { global, .. } => global.push(pkg),
		}
	}

	/// Adds a package to the client list
	pub fn add_client_package(&mut self, pkg: PackageConfigDeser) {
		match self {
			Self::Simple(global) => {
				*self = Self::Full {
					global: global.clone(),
					client: vec![pkg],
					server: Vec::new(),
				}
			}
			Self::Full { client, .. } => client.push(pkg),
		}
	}

	/// Adds a package to the server list
	pub fn add_server_package(&mut self, pkg: PackageConfigDeser) {
		match self {
			Self::Simple(global) => {
				*self = Self::Full {
					global: global.clone(),
					client: Vec::new(),
					server: vec![pkg],
				}
			}
			Self::Full { server, .. } => server.push(pkg),
		}
	}
}

/// Consolidates template configs into the full templates
pub fn consolidate_template_configs(
	templates: HashMap<TemplateID, TemplateConfig>,
	base_template: Option<&TemplateConfig>,
) -> anyhow::Result<HashMap<TemplateID, TemplateConfig>> {
	let mut out: HashMap<_, TemplateConfig> = HashMap::with_capacity(templates.len());

	let max_iterations = 10000;

	// We do this by repeatedly finding a template with an already resolved ancenstor
	let mut i = 0;
	while out.len() != templates.len() {
		for (id, template) in &templates {
			// Don't redo templates that are already done
			if out.contains_key(id) {
				continue;
			}

			if template.instance.common.from.is_empty() {
				// Templates with no ancestor can just be added directly to the output, after deriving from the base template
				let mut template = template.clone();
				if let Some(base_template) = base_template {
					let overlay = template;
					template = base_template.clone();
					template.merge(overlay);
				}
				out.insert(id.clone(), template);
			} else {
				for parent in template.instance.common.from.iter() {
					// If the parent is already in the map (already consolidated) then we can derive from it and add to the map
					let parent_id = TemplateID::from(parent.clone());
					if let Some(parent) = out.get(&parent_id) {
						let mut new = parent.clone();
						new.merge(template.clone());
						out.insert(id.clone(), new);
					} else {
						// Check if the parent template actually doesn't exist or if we just haven't consolidated it yet
						if !templates.contains_key(&parent_id) {
							bail!("Parent template '{parent}' does not exist");
						}
					}
				}
			}
		}

		i += 1;
		if i > max_iterations {
			bail!(
				"Max iterations exceeded while resolving templates. You likely have cyclic templates."
			);
		}
	}

	Ok(out)
}

#[cfg(test)]
mod tests {
	use nitro_shared::util::DeserListOrSingle;

	use crate::instance::CommonInstanceConfig;

	use super::*;

	/// Make sure that consolidated templates are not removed
	#[test]
	fn test_consolidated_still_exists() {
		let mut templates = HashMap::new();
		templates.insert(TemplateID::from("foo"), TemplateConfig::default());
		templates.insert(
			TemplateID::from("bar"),
			TemplateConfig {
				instance: InstanceConfig {
					common: CommonInstanceConfig {
						from: DeserListOrSingle::Single("foo".into()),
						..Default::default()
					},
					..Default::default()
				},
				..Default::default()
			},
		);

		// Ensure determinism
		for _ in 0..30 {
			let consolidated = consolidate_template_configs(templates.clone(), None)
				.expect("Failed to consolidte");
			assert!(consolidated.contains_key(&TemplateID::from("foo")));
			assert!(consolidated.contains_key(&TemplateID::from("bar")));
		}
	}
}
