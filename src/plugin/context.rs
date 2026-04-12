use std::{collections::HashMap, sync::Arc};

use nitro_config::{instance::InstanceConfig, template::TemplateConfig};
use nitro_plugin::host::PluginContext;
use nitro_shared::output::NoOp;

use crate::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	io::paths::Paths,
	plugin::PluginManager,
};

/// Context for the inner plugin manager
pub struct NitroPluginContext {
	pub(crate) instances: Arc<HashMap<String, InstanceConfig>>,
	pub(crate) templates: Arc<HashMap<String, TemplateConfig>>,
	pub(crate) paths: Paths,
	pub(crate) plugins: PluginManager,
}

#[async_trait::async_trait]
impl PluginContext for NitroPluginContext {
	fn get_instances(&self) -> Arc<HashMap<String, InstanceConfig>> {
		self.instances.clone()
	}

	fn get_templates(&self) -> Arc<HashMap<String, TemplateConfig>> {
		self.templates.clone()
	}

	async fn create_instance(&self, id: String, config: InstanceConfig) -> anyhow::Result<()> {
		let mut raw_config = Config::open(&Config::get_path(&self.paths))?;

		let modifications = vec![ConfigModification::AddInstance(id.into(), config)];

		apply_modifications_and_write(
			&mut raw_config,
			modifications,
			&self.paths,
			&self.plugins,
			&mut NoOp,
		)
		.await
	}

	async fn create_template(&self, id: String, config: TemplateConfig) -> anyhow::Result<()> {
		let mut raw_config = Config::open(&Config::get_path(&self.paths))?;

		let modifications = vec![ConfigModification::AddTemplate(id.into(), config)];

		apply_modifications_and_write(
			&mut raw_config,
			modifications,
			&self.paths,
			&self.plugins,
			&mut NoOp,
		)
		.await
	}
}
