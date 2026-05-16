use std::collections::HashMap;
use std::env::current_dir;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::Parser;
use itertools::Itertools;
use nitro_plugin::api::wasm::output::WASMPluginOutput;
use nitro_plugin::api::wasm::{WASMPlugin, sys::get_os_string};
use nitro_plugin::nitro_wasm_plugin;
use nitro_shared::output::{MessageContents, NitroOutput};
use zip::ZipArchive;

use crate::threat::Threat;

mod threat;

nitro_wasm_plugin!(main, "guardian");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.after_packages_installed(|arg| {
		let Some(dir) = arg.inst_dir else {
			return Ok(());
		};

		let dir = PathBuf::from(dir);

		Ok(())
	})?;
	plugin.subcommand(|arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "guardian" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(arg.args.into_iter().skip(1));
		let cli = Cli::try_parse_from(it)?;

		let mut o = WASMPluginOutput::new();

		match cli.command {
			Subcommand::Scan { file } => {
				let path = PathBuf::from(file);
				let path = if path.is_relative() {
					let working_dir = current_dir()?;
					working_dir.join(path)
				} else {
					path
				};

				if !path.exists() {
					bail!("Path does not exist");
				}

				let possible_threats: Vec<Threat> = load_threats();

				if path.is_file() {
					o.start_process();
					o.display(MessageContents::StartProcess("Scanning".into()));

					let file = File::open(path)?;
					let report =
						scan_jar(file, &possible_threats).context("Failed to scan for threats")?;

					o.display(MessageContents::Success("Scanned".into()));
					o.end_process();

					report.report(&mut o, false);
				} else {
					o.start_process();
					o.display(MessageContents::StartProcess("Scanning".into()));

					let result = scan_dir(&path, &possible_threats)?;

					o.display(MessageContents::Success("Scanned".into()));
					o.end_process();

					for (filename, report) in
						result.into_iter().sorted_by_cached_key(|x| x.1.score())
					{
						if report.threats.is_empty() {
							continue;
						}

						o.display(MessageContents::Header(
							filename.to_string_lossy().to_string(),
						));
						let mut section = o.get_section();
						report.report(&mut *section, true);
					}
				}
			}
		}

		Ok(())
	})?;

	Ok(())
}

fn scan_dir(dir: &Path, possible_threats: &[Threat]) -> anyhow::Result<HashMap<OsString, Report>> {
	let mut out = HashMap::new();
	for entry in dir.read_dir().context("Failed to read dir")? {
		let entry = entry?;

		if !entry.file_name().to_string_lossy().ends_with(".jar") {
			continue;
		}

		let file = File::open(entry.path()).context("Failed to open entry")?;
		let result = scan_jar(file, possible_threats)?;
		out.insert(entry.file_name(), result);
	}

	Ok(out)
}

fn scan_jar(data: impl Read + Seek, possible_threats: &[Threat]) -> anyhow::Result<Report> {
	let mut zip = ZipArchive::new(data).context("Failed to open JAR archive")?;
	let mut read_buf = Vec::new();

	let mut threats = Vec::new();

	for i in 0..zip.len() {
		let mut file = zip.by_index(i).context("Failed to get internal file")?;
		file.read_to_end(&mut read_buf)
			.context("Failed to read internal file")?;

		scan_file(&read_buf, possible_threats, &mut threats);
		read_buf.clear();
	}

	Ok(Report { threats })
}

fn scan_file(file: &[u8], possible_threats: &[Threat], out: &mut Vec<Threat>) {
	let os = get_os_string();

	for threat in possible_threats {
		if !threat.signature.repeat && out.iter().any(|x| x.id == threat.id) {
			continue;
		}

		if threat.signature.matches(file, &os) {
			out.push(threat.clone());
		}
	}
}

fn load_threats() -> Vec<Threat> {
	let main: Vec<Threat> = serde_json::from_slice(include_bytes!("threats/main.json")).unwrap();
	let network: Vec<Threat> =
		serde_json::from_slice(include_bytes!("threats/network.json")).unwrap();
	let secrets: Vec<Threat> =
		serde_json::from_slice(include_bytes!("threats/secrets.json")).unwrap();
	let system: Vec<Threat> =
		serde_json::from_slice(include_bytes!("threats/system.json")).unwrap();

	main.into_iter()
		.chain(network)
		.chain(secrets)
		.chain(system)
		.collect()
}

struct Report {
	threats: Vec<Threat>,
}

impl Report {
	fn report(&self, o: &mut impl NitroOutput, compact: bool) {
		for threat in self
			.threats
			.iter()
			.sorted_by_key(|x| std::cmp::Reverse(x.score))
		{
			threat.output(o, compact);
		}

		o.display(MessageContents::property(
			"Total Score",
			MessageContents::Simple(self.score().to_string()),
		));
	}

	fn score(&self) -> u16 {
		self.threats.iter().map(|x| x.score).sum()
	}
}

#[derive(clap::Parser)]
struct Cli {
	#[command(subcommand)]
	command: Subcommand,
}

#[derive(clap::Subcommand)]
#[command(name = "nitro guardian")]
enum Subcommand {
	#[command(about = "Scan a mod jar")]
	Scan {
		/// The JAR file to scan
		file: String,
	},
}
