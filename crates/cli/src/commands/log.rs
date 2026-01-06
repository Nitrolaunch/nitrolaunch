use std::{
	fmt::Display,
	fs::File,
	io::{BufRead, BufReader},
	path::PathBuf,
};

use anyhow::Context;
use clap::Subcommand;
use color_print::{cprintln, cwrite};
use nitrolaunch::io::logging::get_log_files;

use crate::commands::{call_plugin_subcommand, CmdData};

#[derive(Debug, Subcommand)]
pub enum LogSubcommand {
	#[command(about = "Browse log files")]
	Browse {
		/// The client ID to browse files from, either cli or gui. Defaults to cli
		#[arg(short, long)]
		client_id: Option<String>,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

pub async fn run(subcommand: LogSubcommand, data: &mut CmdData<'_>) -> anyhow::Result<()> {
	match subcommand {
		LogSubcommand::Browse { client_id } => browse(data, client_id).await,
		LogSubcommand::External(args) => call_plugin_subcommand(args, Some("log"), data).await,
	}
}

pub async fn browse(data: &mut CmdData<'_>, client_id: Option<String>) -> anyhow::Result<()> {
	let client_id = client_id.unwrap_or("cli".into());
	let logs = get_log_files(&data.paths, &client_id).context("Failed to get log files")?;

	let browse_entries: Vec<_> = logs.iter().map(|x| BrowseEntry::new(x.clone())).collect();

	loop {
		let select = inquire::Select::new(
			"Browsing logs. Press Escape to exit.",
			browse_entries.clone(),
		);
		let log = select.prompt_skippable()?;
		if let Some(log) = log {
			if let Ok(log_text) = std::fs::read_to_string(&log.log_path) {
				cprintln!("<s>Log <g>{}", log.log_path.to_string_lossy());
				println!("{log_text}");
			} else {
				cprintln!("<s,r>Failed to read log {}", log.log_path.to_string_lossy());
			}
			inquire::Confirm::new("Press Escape to return to browse page").prompt_skippable()?;
		} else {
			break;
		}
	}

	Ok(())
}

#[derive(Clone)]
struct BrowseEntry {
	log_path: PathBuf,
	first_log_line: Option<String>,
}

impl BrowseEntry {
	fn new(log_path: PathBuf) -> Self {
		let Ok(log_file) = File::open(&log_path) else {
			return Self {
				log_path,
				first_log_line: None,
			};
		};

		let Some(log_line) = BufReader::new(log_file).lines().next() else {
			return Self {
				log_path,
				first_log_line: None,
			};
		};

		Self {
			log_path,
			first_log_line: log_line.ok(),
		}
	}
}

impl Display for BrowseEntry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(first_log_line) = &self.first_log_line {
			cwrite!(
				f,
				"<s>[<g>{}</>]</> - {first_log_line}",
				self.log_path
					.file_name()
					.unwrap_or_default()
					.to_string_lossy()
			)
		} else {
			cwrite!(
				f,
				"<s>[<g>{}</>]</>",
				self.log_path
					.file_name()
					.unwrap_or_default()
					.to_string_lossy()
			)
		}
	}
}
