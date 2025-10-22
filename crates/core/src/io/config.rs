use std::{collections::HashMap, sync::LazyLock};

use crate::io::{home_dir, json_from_file};

/// Global IO configuration
pub static IO_CONFIG: LazyLock<IoConfig> = LazyLock::new(|| {
	IoConfig::new().unwrap_or_else(|e| {
		eprintln!("Failed to load IO config from file: {e:?}");
		IoConfig::default()
	})
});

/// Global IO configuration
#[derive(Default)]
pub struct IoConfig {
	/// Configuration in key-value form from the file
	file_values: HashMap<String, String>,
}

impl IoConfig {
	fn new() -> anyhow::Result<Self> {
		let path = home_dir()?.join(".nitro.json");

		if !path.exists() {
			Ok(Self::default())
		} else {
			let data: HashMap<String, String> = json_from_file(path)?;

			Ok(Self { file_values: data })
		}
	}

	/// Gets the value of a single config option from either the file or environment. The key should be lower_case.
	pub fn get(&self, key: &str) -> Option<String> {
		if let Ok(value) = std::env::var(format!("NITRO_{}", key.to_ascii_uppercase())) {
			Some(value)
		} else {
			self.file_values.get(key).cloned()
		}
	}
}
