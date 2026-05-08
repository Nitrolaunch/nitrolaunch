#![warn(missing_docs)]

//! This crate contains utilities for managing instance files and addons
//!
//! # Features:
//!
//! - `schema`: Enable generation of JSON schemas using the `schemars` crate

use std::path::{Path, PathBuf};

use nitro_shared::Side;

/// Immutable content for an instance like mods and resource packs
pub mod addon;
/// Storing versions and content in an instance
pub mod lock;

/// Gets the directory for game files on an instance from it's base directory (instances/foo)
pub fn get_instance_dir(base_dir: &Path, side: Side) -> PathBuf {
	match side {
		Side::Client => base_dir.join(".minecraft"),
		Side::Server => base_dir.to_owned(),
	}
}
