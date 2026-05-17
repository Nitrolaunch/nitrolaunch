use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::Parser;
use itertools::Itertools;
use nitro_plugin::api::wasm::nitro::get_instance_dir;
use nitro_plugin::api::wasm::output::WASMPluginOutput;
use nitro_plugin::api::wasm::sys::{fix_relative_path, get_data_dir};
use nitro_plugin::api::wasm::{WASMPlugin, sys::get_os_string};
use nitro_plugin::hook::hooks::OnInstanceSetupResult;
use nitro_plugin::nitro_wasm_plugin;
use nitro_shared::UpdateDepth;
use nitro_shared::output::{MessageContents, NitroOutput, WriterOutput};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::threat::{Mitigation, Threat, constant_pool_end};

mod threat;

static IGNORED_FILES: &[&str] = &[
	"/fabric.mod.json",
	"/quilt.mod.json",
	"/META-INF/MANIFEST.MF",
];

nitro_wasm_plugin!(main, "guardian");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.on_instance_setup(|arg| {
		if arg.will_update_packages {
			return Ok(OnInstanceSetupResult::default());
		}
		let Some(dir) = arg.inst_dir else {
			return Ok(OnInstanceSetupResult::default());
		};

		let dir = PathBuf::from(dir);
		default_scan(&dir)?;

		Ok(OnInstanceSetupResult::default())
	})?;
	plugin.after_packages_installed(|arg| {
		if arg.update_depth < UpdateDepth::Full {
			return Ok(());
		}
		let Some(dir) = arg.inst_dir else {
			return Ok(());
		};

		let dir = PathBuf::from(dir);
		default_scan(&dir)
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
			Subcommand::Scan { file, instance } => {
				let path = if instance {
					let dir = get_instance_dir(&file)
						.context("Failed to get instance directory")?
						.context("Instance does not exist")?;
					dir.join("mods")
				} else {
					fix_relative_path(PathBuf::from(file))
				};

				if !path.exists() {
					bail!("Path does not exist");
				}

				let possible_threats = Threats::load();

				if path.is_file() {
					o.start_process();
					o.display(MessageContents::StartProcess("Scanning".into()));

					let file = std::fs::read(path)?;
					let report =
						scan_jar(&file, &possible_threats).context("Failed to scan for threats")?;

					o.display(MessageContents::Success("Scanned".into()));
					o.end_process();

					report.report(&mut o, false);
				} else {
					o.start_process();
					let result = scan_dir(&path, &possible_threats, &mut o)?;

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

fn default_scan(inst_dir: &Path) -> anyhow::Result<()> {
	let mut o = WASMPluginOutput::new();
	let possible_threats = Threats::load();

	o.start_process();
	let result = scan_dir(&inst_dir.join("mods"), &possible_threats, &mut o)?;

	o.display(MessageContents::Success("Scanned".into()));
	o.end_process();

	for (filename, report) in result.into_iter().sorted_by_cached_key(|x| x.1.score()) {
		let mitigation = report.mitigation();
		if let Mitigation::Detection = mitigation {
			report.dump();
			bail!("Threat detected in {}", filename.to_string_lossy());
		}
	}

	Ok(())
}

fn scan_dir(
	dir: &Path,
	possible_threats: &Threats,
	o: &mut impl NitroOutput,
) -> anyhow::Result<HashMap<OsString, Report>> {
	let mut results = HashMap::new();

	let count = dir.read_dir()?.count();
	let mut i = 0;
	for entry in dir.read_dir().context("Failed to read dir")? {
		let entry = entry?;

		if !entry.file_name().to_string_lossy().ends_with(".jar") {
			continue;
		}

		let file = std::fs::read(entry.path()).context("Failed to open entry")?;

		let report = scan_jar(&file, &possible_threats)?;
		results.insert(entry.file_name(), report);
		o.display(MessageContents::associated(
			MessageContents::Progress {
				current: i,
				total: count as u32,
			},
			MessageContents::Simple("Scanning for threats".into()),
		));
		i += 1;
	}

	Ok(results)
}

fn scan_jar(data: &[u8], possible_threats: &Threats) -> anyhow::Result<Report> {
	// Check for cached scan
	let cache_file = get_scan_cache_path(data, &possible_threats.hash);
	if let Ok(data) = std::fs::read(&cache_file) {
		if let Ok(report) = serde_json::from_slice(&data) {
			return Ok(report);
		}
	}

	let mut zip = ZipArchive::new(Cursor::new(data)).context("Failed to open JAR archive")?;
	let mut read_buf = Vec::new();

	let mut threats = Vec::new();

	for i in 0..zip.len() {
		let mut file = zip.by_index(i).context("Failed to get internal file")?;
		if file.name().starts_with("/assets/") {
			continue;
		}

		file.read_to_end(&mut read_buf)
			.context("Failed to read internal file")?;

		scan_file(&read_buf, file.name(), possible_threats, &mut threats);
		read_buf.clear();
	}

	let out = Report { threats };

	// Cache data
	if let Some(parent) = cache_file.parent() {
		let _ = std::fs::create_dir_all(parent);
	}
	if let Ok(file) = File::create(cache_file) {
		let _ = serde_json::to_writer(file, &out);
	}

	Ok(out)
}

fn scan_file(file: &[u8], file_name: &str, possible_threats: &Threats, out: &mut Vec<Threat>) {
	if IGNORED_FILES.contains(&file_name) {
		return;
	}

	let constant_pool_end = if file_name.ends_with(".class") {
		constant_pool_end(file)
	} else {
		None
	};

	let mut our_threats = Vec::new();
	for threat in &possible_threats.threats {
		if !threat.signature.repeat && out.iter().any(|x| x.id == threat.id) {
			continue;
		}

		if threat.signature.matches(file, constant_pool_end) {
			out.push(threat.clone());
			our_threats.push(threat.clone());
		}
	}
}

struct Threats {
	threats: Vec<Threat>,
	hash: String,
}

impl Threats {
	fn load() -> Self {
		let main: Vec<Threat> =
			serde_json::from_slice(include_bytes!("threats/main.json")).unwrap();
		let network: Vec<Threat> =
			serde_json::from_slice(include_bytes!("threats/network.json")).unwrap();
		let secrets: Vec<Threat> =
			serde_json::from_slice(include_bytes!("threats/secrets.json")).unwrap();
		let system: Vec<Threat> =
			serde_json::from_slice(include_bytes!("threats/system.json")).unwrap();

		let os = get_os_string();

		let out: Vec<_> = main
			.into_iter()
			.chain(network)
			.chain(secrets)
			.chain(system)
			.filter(|x| x.signature.os.is_empty() || x.signature.os.contains(&os))
			.collect();

		let data = serde_json::to_vec(&out).unwrap();
		let hash = blake3::hash(&data).to_string();

		Self { threats: out, hash }
	}
}

#[derive(Clone, Serialize, Deserialize)]
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

	/// Dumps the report to a file
	fn dump(&self) {
		let path = get_data_dir().join("guardian/report.txt");
		if let Ok(file) = File::create(path) {
			let mut o = WriterOutput(file);
			self.report(&mut o, false);
		}
	}

	fn score(&self) -> u16 {
		self.threats.iter().map(|x| x.score).sum()
	}

	fn mitigation(&self) -> Mitigation {
		Mitigation::from_score(self.score())
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
		/// Whether this is the ID of an instance to scan instead
		#[arg(short, long)]
		instance: bool,
	},
}

/// Gets the path for a cached JAR scan
fn get_scan_cache_path(data: &[u8], threats_hash: &str) -> PathBuf {
	let cache_dir = get_data_dir().join("internal/guardian/scan_cache");
	let hash = blake3::hash(data);
	cache_dir.join(format!("{hash}-{threats_hash}.json"))
}
