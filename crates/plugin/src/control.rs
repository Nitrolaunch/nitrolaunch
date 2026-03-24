use serde::{Deserialize, Serialize};

/// A serializable value with a schema
#[derive(Serialize, Deserialize, Clone)]
pub struct Control {
	/// Serialized field ID of this control. Can have dots to specify nested structure.
	pub id: String,
	/// Display name for this control
	pub name: String,
	/// Schema for this control
	pub schema: ControlSchema,
	/// Optional default value for this control
	#[serde(default)]
	pub default: Option<serde_json::Value>,
	/// Tooltip / description for this control
	#[serde(default)]
	pub description: Option<String>,
	/// CSS color for the control. May not be used.
	#[serde(default)]
	pub color: Option<String>,
	/// Name of the section this control is in
	#[serde(default)]
	pub section: Option<String>,
}

/// Schema of possible values and the interface for a controllable value, like a config field
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ControlSchema {
	/// True / False
	Boolean,
	/// Enumeration / Multiple choice
	Choice {
		/// The available variants
		variants: Vec<Variant>,
		/// Whether to allow a null value
		#[serde(default)]
		allow_none: bool,
		/// Whether to use a dropdown
		#[serde(default)]
		dropdown: bool,
		/// Whether selection of multiple variants is allowed
		#[serde(default)]
		multiple: bool,
	},
	/// String of characters
	String {
		/// Whether to force the string to be lowercase
		#[serde(default)]
		lowercase: bool,
	},
	/// Absolute filesystem path
	Path,
	/// Number
	Number {
		/// Minimum value
		min: Option<f32>,
		/// Maximum value
		max: Option<f32>,
		/// Step value. If 1, number will be serialized as an integer.
		step: f32,
		/// Whether to use a slider for the value
		#[serde(default)]
		slider: bool,
	},
	// /// Optional guard around another control
	// Optional(Box<ControlSchema>),
	/// List of sub-objects
	List {
		/// Controls for each sub-object
		fields: Vec<Control>,
		/// Whether this is a schema of a list or string key-value map
		#[serde(default)]
		is_map: bool,
	},
	/// A raw JSON object
	Json,
}

/// Variant of a choice control
#[derive(Serialize, Deserialize, Clone)]

pub struct Variant {
	/// ID of the variant
	pub id: String,
	/// Display name of the variant
	pub name: String,
	/// Color of the variant
	#[serde(default)]
	pub color: Option<String>,
}
