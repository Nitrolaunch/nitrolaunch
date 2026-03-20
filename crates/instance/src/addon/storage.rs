use std::path::{Path, PathBuf};

/// Get the path to a sha256 addon in storage
pub fn get_sha256_addon_path(addons_dir: &Path, hash: &str) -> PathBuf {
	addons_dir.join("sha256").join(hash)
}
