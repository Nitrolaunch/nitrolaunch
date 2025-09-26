//! Note: The asynchronous functions in this library expect the use of the Tokio runtime and may panic
//! if it is not used

/// Download utilities
pub mod download;
/// Uploading and downloading from filebin.net
pub mod filebin;
/// GitHub releases API
pub mod github;
/// Interacting with the Modrinth API
pub mod modrinth;
/// Downloading the NeoForge installer
pub mod neoforge;
/// Interacting with the Smithed API
pub mod smithed;
