use std::path::{Path, PathBuf};

use crate::io::config::IO_CONFIG;

/// IO configuration
pub mod config;

/// Tries to get the user's home dir
pub fn home_dir() -> anyhow::Result<PathBuf> {
	#[cfg(target_os = "linux")]
	let path = std::env::var("HOME")?;
	#[cfg(target_os = "windows")]
	let path = format!("{}/..", std::env::var("%APPDATA%")?);
	#[cfg(target_os = "macos")]
	let path = std::env::var("HOME")?;
	#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
	let path = "/";

	Ok(PathBuf::from(path))
}

/// Gets the configured IO link method
pub fn get_link_method() -> LinkMethod {
	let method = IO_CONFIG.get_string("link_method");
	let Some(method) = method else {
		#[cfg(target_family = "unix")]
		return LinkMethod::Soft;
		// Soft links require admin priveleges on Windows
		#[cfg(target_os = "windows")]
		return LinkMethod::Hard;
		#[cfg(not(any(target_os = "windows", target_family = "unix")))]
		return LinkMethod::Soft;
	};

	match method.as_str() {
		"hard" => LinkMethod::Hard,
		"soft" => LinkMethod::Soft,
		"copy" => LinkMethod::Copy,
		_ => LinkMethod::Hard,
	}
}

/// Different methods for files to be linked with
pub enum LinkMethod {
	/// Hardlink
	Hard,
	/// Symlink
	Soft,
	/// File is copied
	Copy,
}

/// Creates a new link if it does not exist
pub fn update_link(path: &Path, link: &Path) -> std::io::Result<()> {
	let method = get_link_method();

	match method {
		LinkMethod::Hard => {
			if !link.exists() {
				std::fs::hard_link(path, link)?;
			}
		}
		LinkMethod::Soft => {
			if !link.exists() {
				#[allow(deprecated)]
				std::fs::soft_link(path, link)?;
			}
		}
		LinkMethod::Copy => {
			if !link.exists() {
				std::fs::copy(path, link)?;
			}
		}
	}

	Ok(())
}
