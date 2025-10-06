use std::collections::HashMap;

use anyhow::{bail, Context};
use nitro_plugin::api::CustomPlugin;
use nitro_shared::id::InstanceID;
use nitrolaunch::config_crate::instance::InstanceConfig;
use serde::{Deserialize, Serialize};

/// Replacement token for the index of the current multiplied instance
static INDEX_TOKEN: &str = "$n";

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("multiply", include_str!("plugin.json"))?;

	plugin.add_instances(|ctx, _| {
		let Some(config) = ctx.get_custom_config() else {
			return Ok(HashMap::new());
		};

		let config: MultiplyConfig =
			serde_json::from_str(config).context("Invalid Multiply config")?;

		let mut out = HashMap::new();

		for (id, config) in config.instances {
			if !id.contains(INDEX_TOKEN) {
				bail!("Template {id} is missing the index token ({INDEX_TOKEN}) to make each copy unique");
			}
			multiply(id, config, &mut out);
		}

		Ok(out)
	})?;

	Ok(())
}

#[derive(Serialize, Deserialize, Default)]
struct MultiplyConfig {
	instances: HashMap<String, MultiplyInstance>,
}

#[derive(Serialize, Deserialize)]
struct MultiplyInstance {
	/// The start point for the number
	#[serde(default)]
	start: u16,
	/// The number of times to multiply
	count: u16,
	#[serde(flatten)]
	config: InstanceConfig,
}

/// Multiplies an instance
fn multiply(id: String, multiply: MultiplyInstance, out: &mut HashMap<InstanceID, InstanceConfig>) {
	let Ok(config_value) = serde_json::to_value(multiply.config) else {
		return;
	};

	for i in multiply.start..multiply.count {
		let i = i.to_string();

		let id = id.replace(INDEX_TOKEN, &i);

		let mut config_value = config_value.clone();
		replace_index_tokens(&mut config_value, &i);

		let Ok(config) = serde_json::from_value(config_value) else {
			continue;
		};

		out.insert(id.into(), config);
	}
}

/// Replaces index tokens in every string field of a value
fn replace_index_tokens(value: &mut serde_json::Value, n: &str) {
	match value {
		serde_json::Value::Array(values) => {
			for value in values {
				replace_index_tokens(value, n);
			}
		}
		serde_json::Value::Object(props) => {
			for prop in props.values_mut() {
				replace_index_tokens(prop, n);
			}
		}
		serde_json::Value::String(value) => {
			*value = value.replace(INDEX_TOKEN, n);
		}
		_ => {}
	}
}
