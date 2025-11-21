#![warn(missing_docs)]

//! This library is used by both Nitrolaunch to load plugins, and as a framework for defining
//! Rust plugins for Nitrolaunch to use

use std::path::PathBuf;

/// API for Rust-based plugins to use
pub mod api;
/// Plugin hooks and calling them
pub mod hook;
/// System for hosting / running plugins
#[cfg(feature = "host")]
pub mod host;
/// Serialized output format for plugins
pub mod input_output;
/// Plugins
#[cfg(feature = "host")]
pub mod plugin;
/// Tokio helpers for AsyncRead
pub mod try_read;

pub use nitro_shared as shared;

/// Environment variable that debugs plugins when set
pub static PLUGIN_DEBUG_ENV: &str = "NITRO_PLUGIN_DEBUG";

/// Gets whether plugin debugging is enabled
pub fn plugin_debug_enabled() -> bool {
	std::env::var(PLUGIN_DEBUG_ENV).unwrap_or_default() == "1"
}

/// Paths for the plugin crate
pub struct PluginPaths {
	/// Data directory
	pub data_dir: PathBuf,
	/// Config directory
	pub config_dir: PathBuf,
}

/// A pointer and length, usually a string or byte buffer, used for WASM code
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PtrAndLength {
	/// The WASM memory pointer
	pub ptr: u32,
	/// The buffer length
	pub len: u32,
}

impl PtrAndLength {
	/// Returns a null pointer
	pub fn null() -> Self {
		Self { ptr: 0, len: 0 }
	}

	/// Gets this pointer and length as a tuple
	pub fn tuple(&self) -> (u32, u32) {
		(self.ptr, self.len)
	}
}
