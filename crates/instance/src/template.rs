use nitro_config::template::TemplateConfig;
use nitro_shared::id::TemplateID;
use serde::{Deserialize, Serialize};

/// The name of the template data file inside a template
pub static TEMPLATE_DATA_JSON: &str = "template.json";

/// Structure for the template.json file inside a template
#[derive(Serialize, Deserialize, Clone)]
pub struct TemplateData {
	/// Unique identifier for the template
	pub id: TemplateID,
	/// Configuration for instances using this template
	pub config: TemplateConfig,
}
