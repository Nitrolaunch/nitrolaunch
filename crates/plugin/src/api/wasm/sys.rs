use std::path::PathBuf;

/// Gets the Nitrolaunch data directory
pub fn get_data_dir() -> PathBuf {
	PathBuf::from(super::interface::get_data_dir())
}

/// Gets the Nitrolaunch config directory
pub fn get_config_dir() -> PathBuf {
	PathBuf::from(super::interface::get_config_dir())
}

/// Gets the operating system as a lowercase string
pub fn get_os_string() -> String {
	super::interface::get_os_string()
}

/// Gets the system architecture as a lowercase string
pub fn get_arch_string() -> String {
	super::interface::get_arch_string()
}

/// Gets the system pointer width
pub fn get_pointer_width() -> u32 {
	super::interface::get_pointer_width()
}
