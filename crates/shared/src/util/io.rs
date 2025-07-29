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
