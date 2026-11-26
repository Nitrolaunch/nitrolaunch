use std::{path::PathBuf, sync::LazyLock};

use nitro_config::template::TemplateConfig;
use nitro_shared::id::TemplateID;
use serde::{Deserialize, Serialize};

/// The name of the template data file inside a template
pub static TEMPLATE_DATA_JSON: &str = "template.json";

/// Built-in file component format
pub static BUILTIN_FILE_FORMAT: LazyLock<ComponentFormat> = LazyLock::new(|| ComponentFormat {
	id: "file".into(),
	name: "File".into(),
	plugin: None,
	aliases: Vec::new(),
});

/// Built-in JSON component format
pub static BUILTIN_JSON_FORMAT: LazyLock<ComponentFormat> = LazyLock::new(|| ComponentFormat {
	id: "json".into(),
	name: "JSON".into(),
	plugin: None,
	aliases: Vec::new(),
});

/// Structure for the template.json file inside a template
#[derive(Serialize, Deserialize, Clone)]
pub struct TemplateData {
	/// Unique identifier for the template
	pub id: TemplateID,
	/// Configuration for instances using this template
	pub config: TemplateConfig,
	/// Registration for components in the template
	pub components: Vec<TemplateComponent>,
}

/// Registered component in a template
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TemplateComponent {
	/// Path to the component, interpreted however the format of the component wants
	pub path: String,
	/// Format of the component
	pub format: String,
	/// The plugin where the format is from
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub plugin: Option<String>,
	/// Display name of the component
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// BLAKE3 hash of the component, stored in hex
	pub hash: String,
}

impl TemplateComponent {
	/// Reads the hash of the component
	pub fn hash(&self) -> Option<[u8; 32]> {
		let mut out = [0u8; 32];
		hex::decode_to_slice(&self.hash, &mut out).ok()?;
		Some(out)
	}

	/// Gets the full path of the component, including the plugin and format
	pub fn full_path(&self) -> String {
		if let Some(plugin) = &self.plugin {
			format!("{plugin}/{}/{}", self.format, self.path)
		} else {
			format!("builtin/{}/{}", self.format, self.path)
		}
	}

	/// Splits the path of the component into location and file parts, before and after the first colon
	pub fn path_parts(&self) -> (&str, &str) {
		path_parts(&self.path)
	}

	/// Checks if this component is fully equal to another component, including the hash
	pub fn fully_equal(&self, other: &Self) -> bool {
		self == other && self.hash == other.hash
	}
}

impl PartialEq for TemplateComponent {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path && self.format == other.format && self.plugin == other.plugin
	}
}

impl Eq for TemplateComponent {}

/// Component actualized / found in an instance
#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceComponent {
	/// Path to the component, interpreted however the format of the component wants
	pub path: String,
	/// Display name of the component
	#[serde(default)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// Children of this component
	pub children: Vec<String>,
}

impl InstanceComponent {
	/// Creates a new InstanceComponent with the given path and no children
	pub fn new(path: String) -> Self {
		Self {
			path,
			name: None,
			children: Vec::new(),
		}
	}

	/// Converts this InstanceComponent into a TemplateComponent with the given format, plugin, and hash
	pub fn to_template_component(
		&self,
		format: String,
		plugin: Option<String>,
		hash: String,
	) -> TemplateComponent {
		TemplateComponent {
			path: self.path.clone(),
			format,
			plugin,
			name: self.name.clone(),
			hash,
		}
	}

	/// Splits the path of the component into location and file parts, before and after the first colon
	pub fn path_parts(&self) -> (&str, &str) {
		path_parts(&self.path)
	}
}

impl PartialEq for InstanceComponent {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path
	}
}

impl Eq for InstanceComponent {}

/// Component with source directory, used for applying components
#[derive(Serialize, Deserialize, Clone)]
pub struct ComponentWithSource {
	/// The component
	pub component: TemplateComponent,
	/// The source directory of the component
	pub source_dir: PathBuf,
}

/// A format for a component
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ComponentFormat {
	/// The format ID of the component format
	pub id: String,
	/// Display name of the component format
	pub name: String,
	/// The plugin where the format is from
	pub plugin: Option<String>,
	/// Aliases for the format ID, used to make typing easier for users
	pub aliases: Vec<String>,
}

/// Merge mode for components when applying a component to a destination
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComponentMergeMode {
	/// Merge the component underneath existing destination components
	#[default]
	MergeUnder,
	/// Merge the component on top of existing destination components
	MergeOver,
	/// Replace the existing destination components with the new component
	Replace,
	/// Do not apply the component if there are existing destination components
	Skip,
}

/// Determines the direction of applying a component to a destination
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentApplyMode {
	/// We are saving the component to storage. It can be saved in a more compact format and does not have to be game-readable.
	Save,
	/// We are restoring the component from storage. It must be in a game-readable format.
	Restore,
}

fn path_parts(path: &str) -> (&str, &str) {
	path.split_once(':').unwrap_or((path, ""))
}
