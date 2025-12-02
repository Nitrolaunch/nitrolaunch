use std::path::PathBuf;

/// Prevents a command from spawning a window on Windows
///
/// Used from https://github.com/Mrmayman/quantumlauncher/
#[macro_export]
macro_rules! no_window {
	($cmd:expr) => {
		#[cfg(target_os = "windows")]
		{
			use std::os::windows::process::CommandExt;
			// 0x08000000 => CREATE_NO_WINDOW
			$cmd.creation_flags(0x08000000);
		}
	};
}

/// Replaces a tilde in a string path
pub fn replace_tilde(path: &str) -> PathBuf {
	#[cfg(target_family = "unix")]
	let path = if path.starts_with("~") {
		if let Ok(home) = std::env::var("HOME") {
			home + &path[1..]
		} else {
			path.to_string()
		}
	} else {
		path.to_string()
	};

	PathBuf::from(path)
}
