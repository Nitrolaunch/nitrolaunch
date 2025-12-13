use std::{
	ffi::OsStr,
	path::{Path, PathBuf},
};

use anyhow::anyhow;

/// Gets the Nitrolaunch data directory
pub fn get_data_dir() -> PathBuf {
	PathBuf::from(super::interface::get_data_dir())
}

/// Gets the Nitrolaunch config directory
pub fn get_config_dir() -> PathBuf {
	PathBuf::from(super::interface::get_config_dir())
}

/// Gets the current working directory
pub fn get_current_dir() -> PathBuf {
	PathBuf::from(super::interface::get_current_dir())
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

/// Updates a hardlink between two files
pub fn update_hardlink(src: impl AsRef<Path>, tgt: impl AsRef<Path>) -> anyhow::Result<()> {
	super::interface::update_hardlink(
		&src.as_ref().to_string_lossy(),
		&tgt.as_ref().to_string_lossy(),
	)
	.map_err(|e| anyhow!("{e:?}"))
}

/// Runs a command
pub fn run_command(
	cmd: impl AsRef<OsStr>,
	args: Vec<impl AsRef<OsStr>>,
	working_dir: Option<impl AsRef<OsStr>>,
	suppress_command_window: bool,
	silent: bool,
	wait: bool,
) -> anyhow::Result<i32> {
	let args: Vec<_> = args
		.into_iter()
		.map(|x| x.as_ref().to_string_lossy().to_string())
		.collect();

	super::interface::run_command(
		&cmd.as_ref().to_string_lossy(),
		&args,
		working_dir
			.map(|x| x.as_ref().to_string_lossy().to_string())
			.as_deref(),
		suppress_command_window,
		silent,
		wait,
	)
	.map_err(|e| anyhow!("{e:?}"))
}
