use std::{fs::File, path::Path};

use anyhow::{bail, Context};
use itertools::Itertools;
use libflate::gzip::Decoder;

/// Gets the list of log file names in the given Minecraft logs dir ordered from oldest to newest
pub fn list_logs(logs_dir: &Path) -> anyhow::Result<Vec<String>> {
	if !logs_dir.exists() {
		return Ok(Vec::new());
	}

	let read = logs_dir
		.read_dir()
		.context("Failed to read logs directory")?;

	let mapped = read.filter_map(|x| {
		let x = x.ok()?;
		if !x.file_type().ok()?.is_file() {
			return None;
		}

		let name = x.file_name();
		if !name.to_string_lossy().contains(".log.gz") {
			return None;
		}

		let time = x.metadata().ok()?.created().ok()?;

		Some((x.file_name(), time))
	});

	let sorted = mapped
		.sorted_by_key(|x| x.1)
		.map(|x| x.0.to_string_lossy().to_string());

	Ok(sorted.collect())
}

/// Attempts to read the text from the given log file
pub fn read_log(path: &Path) -> anyhow::Result<String> {
	if !path.is_file() {
		bail!("Log is not a file");
	}

	let filename = path.file_name().unwrap().to_string_lossy().to_string();
	let file = File::open(path).context("Failed to open log file")?;
	if filename.ends_with(".gz") {
		let archive = Decoder::new(file)?;
		std::io::read_to_string(archive).context("Failed to read archived log")
	} else {
		std::io::read_to_string(file).context("Failed to read log")
	}
}
