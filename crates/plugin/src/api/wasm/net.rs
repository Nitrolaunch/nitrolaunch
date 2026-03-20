use std::path::Path;

use anyhow::anyhow;

/// Downloads bytes from the given URL
pub fn download_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
	super::interface::download_bytes(url).map_err(|e| anyhow!("{e}"))
}

/// Downloads text from the given URL
pub fn download_text(url: &str) -> anyhow::Result<String> {
	super::interface::download_text(url).map_err(|e| anyhow!("{e}"))
}

/// Downloads a file from the given URL to the target path
pub fn download_file(url: &str, path: impl AsRef<Path>) -> anyhow::Result<()> {
	super::interface::download_file(url, &path.as_ref().to_string_lossy())
		.map_err(|e| anyhow!("{e}"))
}

/// Downloads multiple files
pub fn download_files(
	urls: &[String],
	paths: &[String],
	skip_existing: bool,
) -> anyhow::Result<()> {
	super::interface::download_files(urls, paths, skip_existing).map_err(|e| anyhow!("{e}"))
}
