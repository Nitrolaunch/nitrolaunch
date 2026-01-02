use std::{fs::File, io::Write, path::PathBuf};

use anyhow::Context;
use itertools::Itertools;
use nitro_shared::{
	output::{MessageContents, MessageLevel},
	util::utc_timestamp,
};

use crate::io::paths::Paths;

/// A logging struct used to write output messages to log files
pub struct Logger {
	log_file: Option<File>,
	latest_log_file: Option<File>,
}

impl Logger {
	/// Opens this logger with a client ID (cli, gui, etc) to identify the log files
	pub fn new(paths: &Paths, client_id: &str) -> anyhow::Result<Self> {
		let _ = clear_old_logs(paths);
		let path = get_log_file_path(paths, client_id).context("Failed to get log file path")?;
		let log_file = File::create(path).context("Failed to open log file")?;
		let latest_log_file = File::create(get_latest_log_file_path(paths)).ok();

		Ok(Self {
			log_file: Some(log_file),
			latest_log_file,
		})
	}

	/// Creates a dummy logger that doesn't log anything
	pub fn dummy() -> Self {
		Self {
			log_file: None,
			latest_log_file: None,
		}
	}

	/// Writes a log message
	pub fn log_message(
		&mut self,
		message: MessageContents,
		level: MessageLevel,
	) -> anyhow::Result<()> {
		let message = format_log_message(&format_log_message_contents(message), level) + "\n";

		if let Some(log_file) = &mut self.log_file {
			log_file.write_all(message.as_bytes())?;
		}
		if let Some(latest_log_file) = &mut self.latest_log_file {
			latest_log_file.write_all(message.as_bytes())?;
		}

		Ok(())
	}
}

/// Get the path to a new log file
pub fn get_log_file_path(paths: &Paths, client_id: &str) -> anyhow::Result<PathBuf> {
	Ok(paths
		.logs
		.join(format!("log-{client_id}-{}.txt", utc_timestamp()?)))
}

/// Get the path to the latest log file
pub fn get_latest_log_file_path(paths: &Paths) -> PathBuf {
	paths.logs.join("latest.txt")
}

/// Formats a full log message
pub fn format_log_message(text: &str, level: MessageLevel) -> String {
	let level_indicator = match level {
		MessageLevel::Important => "info",
		MessageLevel::Extra => "extra",
		MessageLevel::Debug => "debug",
		MessageLevel::Trace => "trace",
	};

	format!("[{level_indicator}] {text}")
}

/// Formats a MessageContents for a log message
pub fn format_log_message_contents(contents: MessageContents) -> String {
	match contents {
		MessageContents::Simple(text) => text,
		MessageContents::Notice(text) => format!("[NOTICE] {}", text),
		MessageContents::Warning(text) => format!("[WARN] {}", text),
		MessageContents::Error(text) => format!("[ERR] {}", text),
		MessageContents::Success(text) => format!("[SUCCESS] {}", add_period(text)),
		MessageContents::Property(key, value) => {
			format!("{}: {}", key, format_log_message_contents(*value))
		}
		MessageContents::Header(text) => format!("### {} ###", text),
		MessageContents::StartProcess(text) => format!("{text}..."),
		MessageContents::Associated(item, message) => {
			format!(
				"({}) {}",
				format_log_message_contents(*item),
				format_log_message_contents(*message)
			)
		}
		MessageContents::Package(pkg, message) => {
			let pkg_disp = pkg.debug_sources();
			format!("[{}] {}", pkg_disp, format_log_message_contents(*message))
		}
		MessageContents::Hyperlink(url) => url,
		MessageContents::ListItem(item) => " - ".to_string() + &format_log_message_contents(*item),
		MessageContents::Copyable(text) => text,
		MessageContents::Progress { current, total } => format!("{current}/{total}"),
		contents => contents.default_format(),
	}
}

/// Clears out old log files
pub fn clear_old_logs(paths: &Paths) -> anyhow::Result<()> {
	let dir_reader = paths
		.logs
		.read_dir()
		.context("Failed to read logs directory")?;

	let mut count = 0;
	let mapped = dir_reader.filter_map(|x| {
		let x = x.ok()?;
		if !x.file_type().ok()?.is_file() {
			return None;
		}

		let name = x.file_name();
		if !name.to_string_lossy().contains("log-") {
			return None;
		}
		let time = x.metadata().ok()?.created().ok()?;

		count += 1;

		Some((name, time))
	});
	// Sort so that the oldest are at the end
	let sorted = mapped.sorted_by_cached_key(|x| x.1).rev();
	if count > 15 {
		for (name, _) in sorted.skip(15) {
			let _ = std::fs::remove_file(paths.logs.join(name));
		}
	}

	Ok(())
}

/// Adds a period to the end of a string if it isn't punctuated already
fn add_period(string: String) -> String {
	if string.ends_with(['.', ',', ';', ':', '!', '?']) {
		string
	} else {
		string + "."
	}
}
