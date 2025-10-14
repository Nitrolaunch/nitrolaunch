use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek};
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::Serialize;
use zip::ZipArchive;

/// Global IO configuration using a file or environment variables
pub mod config;
/// Utilities for dealing with the filesystem
pub mod files;
/// Interaction with some of Java's formats
pub mod java;
/// I/O with Minecraft data formats
pub mod minecraft;
/// Use of a file for persistent data
pub mod persistent;
/// Management of file updates
pub mod update;

/// Reads JSON from a file with a buffer
pub fn json_from_file<D: DeserializeOwned>(path: impl AsRef<Path>) -> anyhow::Result<D> {
	let file = BufReader::new(File::open(path).context("Failed to open file")?);
	Ok(simd_json::from_reader(file)?)
}

/// Writes JSON to a file with a buffer
pub fn json_to_file<S: Serialize>(path: impl AsRef<Path>, data: &S) -> anyhow::Result<()> {
	let file = BufWriter::new(File::create(path).context("Failed to open file")?);
	simd_json::to_writer(file, data).context("Failed to serialize data to file")?;
	Ok(())
}

/// Writes JSON to a file with a buffer and pretty formatting
pub fn json_to_file_pretty<S: Serialize>(path: impl AsRef<Path>, data: &S) -> anyhow::Result<()> {
	let file = BufWriter::new(File::create(path).context("Failed to open file")?);
	serde_json::to_writer_pretty(file, data).context("Failed to serialize data to file")?;
	Ok(())
}

/// Writes JSON to a file with less than ideal formatting, but at a higher speed
pub fn json_to_file_pretty_fast<S: Serialize>(
	path: impl AsRef<Path>,
	data: &S,
) -> anyhow::Result<()> {
	let file = BufWriter::new(File::create(path).context("Failed to open file")?);
	simd_json::to_writer_pretty(file, data).context("Failed to serialize data to file")?;
	Ok(())
}

/// Extracts a specific directory within a zip file
pub fn extract_zip_dir<R: Read + Seek>(
	zip: &mut ZipArchive<R>,
	zip_dir: &str,
	target_dir: impl AsRef<Path>,
) -> anyhow::Result<()> {
	let _ = std::fs::create_dir_all(target_dir.as_ref());

	for index in 0..zip.len() {
		let mut file = zip.by_index(index)?;
		if file.is_dir() {
			continue;
		}

		let Some(filename) = file.enclosed_name() else {
			continue;
		};

		let Ok(filename) = filename.strip_prefix(zip_dir) else {
			continue;
		};

		let out_path = target_dir.as_ref().join(filename);
		let _ = files::create_leading_dirs(&out_path);

		let mut out_file = File::create(out_path).context("Failed to create output file")?;

		std::io::copy(&mut file, &mut out_file).context("Failed to copy file from zip")?;
	}

	Ok(())
}

/// Tries to get the user's home dir
pub fn home_dir() -> anyhow::Result<PathBuf> {
	#[cfg(target_os = "linux")]
	let path = std::env::var("HOME")?;
	#[cfg(target_os = "windows")]
	let path = format!("{}/..", std::env::var("%APPDATA%")?);
	#[cfg(target_os = "macos")]
	let path = std::env::var("HOME")?;

	Ok(PathBuf::from(path))
}
