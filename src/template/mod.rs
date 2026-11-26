use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use anyhow::Context;
use nitro_config::template::TemplateConfig;
use nitro_core::io::{json_from_file, json_to_file};
use nitro_instance::template::{TEMPLATE_DATA_JSON, TemplateData};
use nitro_shared::{
	id::TemplateID,
	output::{MessageContents, NitroOutput},
};

/// Applying and resolving templates on instances
pub mod apply;
/// Installing and launching proxies on templates
pub mod proxy;

/// A template loaded from configuration
pub struct Template {
	data: TemplateData,
	path: PathBuf,
	/// Consolidated configuration. Will be default until set.
	config: TemplateConfig,
}

impl Template {
	/// Gets the path to a template from the templates directory and template ID
	pub fn get_path(templates_dir: &Path, id: &TemplateID) -> PathBuf {
		templates_dir.join(&**id)
	}

	/// Loads a template from a path
	pub fn load(path: &Path) -> anyhow::Result<Self> {
		let data_path = path.join(TEMPLATE_DATA_JSON);
		let data: TemplateData =
			json_from_file(&data_path).context("Failed to load template data file")?;

		Ok(Self {
			data,
			path: path.to_owned(),
			config: TemplateConfig::default(),
		})
	}

	/// Saves the template to disk
	pub fn save(&self) -> anyhow::Result<()> {
		let _ = std::fs::create_dir_all(&self.path);
		let data_path = self.path.join(TEMPLATE_DATA_JSON);
		json_to_file(&data_path, &self.data).context("Failed to save template data file")
	}

	/// Creates a new template from data and a path
	pub fn from_data(data: TemplateData, path: PathBuf) -> Self {
		Self {
			data,
			path,
			config: TemplateConfig::default(),
		}
	}

	/// Creates a new template from a config and a path
	pub fn from_config(id: TemplateID, config: TemplateConfig, path: PathBuf) -> Self {
		let data = TemplateData {
			id,
			config,
			components: Vec::new(),
		};
		Self::from_data(data, path)
	}

	/// Gets the unique identifier for the template
	pub fn id(&self) -> &TemplateID {
		&self.data.id
	}

	/// Gets the unconsolidated configuration for the template
	pub fn original_config(&self) -> &TemplateConfig {
		&self.data.config
	}

	/// Gets the consolidated configuration for the template
	pub fn config(&self) -> &TemplateConfig {
		&self.config
	}

	/// Sets the consolidated configuration for the template
	pub fn set_consolidated_config(&mut self, config: TemplateConfig) {
		self.config = config;
	}
}

/// Loads all templates from a directory and old configuration templates, and migrates old templates to ones on disk
pub fn load_templates(
	templates_dir: &Path,
	old_config_templates: &HashMap<TemplateID, TemplateConfig>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<HashMap<TemplateID, Template>> {
	let mut templates = HashMap::new();

	if templates_dir.exists() {
		for entry in
			std::fs::read_dir(templates_dir).context("Failed to read templates directory")?
		{
			let Ok(entry) = entry else {
				o.display(MessageContents::Warning(
					"Failed to read template entry, skipping".into(),
				));
				continue;
			};
			let Ok(file_type) = entry.file_type() else {
				o.display(MessageContents::Warning(format!(
					"Failed to get template directory entry type for {:?}, skipping",
					entry.path()
				)));
				continue;
			};
			if !file_type.is_dir() {
				continue;
			}
			let Ok(template) = Template::load(&entry.path()) else {
				o.display(MessageContents::Warning(format!(
					"Failed to load template from {:?}, skipping",
					entry.path()
				)));
				continue;
			};
			templates.insert(template.id().clone(), template);
		}
	} else {
		std::fs::create_dir_all(templates_dir).context("Failed to create templates directory")?;
	}

	// Migrate old templates to new ones on disk
	for (id, config) in old_config_templates {
		let template_path = templates_dir.join(&**id);
		if !template_path.exists() {
			o.debug(MessageContents::Simple(format!(
				"Migrating old template {} to new template on disk",
				id
			)));
			let template = Template::from_config(id.clone(), config.clone(), template_path);
			template
				.save()
				.context("Failed to save migrated template")?;
			templates.insert(template.id().clone(), template);
		}
	}

	Ok(templates)
}

/// Gets the storage path for a component in the storage directory based on its hash
pub fn get_component_storage_path(hash: &str, storage_dir: &Path) -> PathBuf {
	storage_dir.join(&hash[0..2]).join(hash)
}
