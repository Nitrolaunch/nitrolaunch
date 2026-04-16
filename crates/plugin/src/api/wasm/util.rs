use serde::{de::DeserializeOwned, Serialize};

/// Gets the custom config for this plugin
pub fn get_custom_config() -> Option<String> {
	super::interface::get_custom_config()
}

/// Gets the persistent state of the plugin
pub fn get_persistent_state<D: DeserializeOwned>() -> Option<D> {
	serde_json::from_str(&super::interface::get_persistent_state()).unwrap()
}

/// Sets the persistent state of the plugin
pub fn set_persistent_state<S: Serialize>(s: &S) {
	super::interface::set_persistent_state(&serde_json::to_string(s).unwrap());
}
