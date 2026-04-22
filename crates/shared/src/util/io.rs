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
	let path = if let Some(stripped) = path.strip_prefix("~") {
		if let Ok(home) = std::env::var("HOME") {
			home + stripped
		} else {
			path.to_string()
		}
	} else {
		path.to_string()
	};

	PathBuf::from(path)
}

/// Utility to automatically run a drop function if the code is not completed successful
/// (often to prevent invalid data from being saved on disk)
pub struct CorruptionGuard<F: FnOnce() -> ()> {
	f: Option<F>,
	is_successful: bool,
}

impl<F: FnOnce() -> ()> CorruptionGuard<F> {
	/// Creates a new corruption guard
	pub fn new(f: F) -> Self {
		Self {
			f: Some(f),
			is_successful: false,
		}
	}

	/// Marks the guard as successful. The drop function will not run.
	pub fn succeed(&mut self) {
		self.is_successful = true;
	}
}

impl<F: FnOnce() -> ()> Drop for CorruptionGuard<F> {
	fn drop(&mut self) {
		if !self.is_successful {
			if let Some(f) = self.f.take() {
				f();
			}
		}
	}
}
