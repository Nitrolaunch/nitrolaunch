/// Downloading essential files for launching the game
pub mod game_files;
/// Downloading different flavors of the JRE
pub mod java;
/// Interacting with the Minecraft / Microsoft / Mojang APIs
pub mod minecraft;

// Re-export
pub use nitro_net::download;

use crate::io::config::IO_CONFIG;

/// Sensible open file descriptor limit for asynchronous transfers
#[cfg(target_os = "windows")]
const FD_SENSIBLE_LIMIT: usize = 128;
/// Sensible open file descriptor limit for asynchronous transfers
#[cfg(not(target_os = "windows"))]
const FD_SENSIBLE_LIMIT: usize = 128;

/// Get the sensible limit for asynchronous transfers
pub fn get_transfer_limit() -> usize {
	if let Some(env) = IO_CONFIG.get("transfer_limit") {
		env.parse().unwrap_or_default()
	} else {
		FD_SENSIBLE_LIMIT
	}
}
