use std::{collections::HashMap, fs::File, sync::LazyLock};

use crate::io::home_dir;

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
	file_values: HashMap<String, serde_json::Value>,
}

impl IoConfig {
	fn new() -> anyhow::Result<Self> {
		let path = home_dir()?.join(".nitro.json");

		if !path.exists() {
			Ok(Self::default())
		} else {
			let file = File::open(path)?;
			let data: HashMap<String, serde_json::Value> = serde_json::from_reader(file)?;

			Ok(Self { file_values: data })
		}
	}

	/// Gets the value of a string property from either the file or environment. The key should be lower_case.
	pub fn get_string(&self, key: &str) -> Option<String> {
		if let Ok(value) = std::env::var(format!("NITRO_{}", key.to_ascii_uppercase())) {
			Some(value)
		} else {
			self.file_values
				.get(key)
				.and_then(|x| x.as_str())
				.map(|x| x.to_string())
		}
	}

	/// Gets the value of a boolean property from either the file or environment. The key should be lower_case.
	pub fn get_bool(&self, key: &str) -> Option<bool> {
		if let Ok(value) = std::env::var(format!("NITRO_{}", key.to_ascii_uppercase())) {
			Some(match value.as_str() {
				"1" => true,
				_ => false,
			})
		} else {
			self.file_values.get(key).and_then(|x| x.as_bool())
		}
	}

	/// Gets the value of an integer property from either the file or environment. The key should be lower_case.
	pub fn get_int(&self, key: &str) -> Option<i64> {
		if let Ok(value) = std::env::var(format!("NITRO_{}", key.to_ascii_uppercase())) {
			value.parse().ok()
		} else {
			self.file_values.get(key).and_then(|x| x.as_i64())
		}
	}
}
