use std::{
	collections::HashMap,
	fmt::Display,
	fs::File,
	io::{BufRead, BufReader},
	ops::DerefMut,
	path::{Path, PathBuf},
	process::Command,
};

use anyhow::{anyhow, bail, Context};
use nitro_core::{
	io::{
		files::create_leading_dirs,
		java::{
			classpath::{Classpath, CLASSPATH_SEP},
			maven::MavenLibraryParts,
		},
		update::UpdateManager,
	},
	net::game_files::{
		client_meta::args::{ArgumentItem, Arguments},
		libraries,
	},
};
use nitro_net::neoforge;
use nitro_shared::{
	output::{MessageContents, MessageLevel, NitroOutput},
	versions::VersionInfo,
	Side, UpdateDepth,
};
use reqwest::Client;
use serde::Deserialize;
use zip::ZipArchive;

/// Mode we are in (Fabric / Quilt)
/// This way we don't have to duplicate a lot of functions since these both
/// have very similar download steps
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
	/// NeoForge loader
	NeoForge,
}

impl Mode {
	/// Convert to a lowercase string
	pub fn to_str(&self) -> &'static str {
		match self {
			Self::NeoForge => "neoforge",
		}
	}
}

impl Display for Mode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::NeoForge => "NeoForge",
			}
		)
	}
}

/// Installs Forge or NeoForge from a version. Returns the classpath as well as JVM and game args
pub async fn install(
	client: &Client,
	internal_dir: &Path,
	update_depth: UpdateDepth,
	version_info: &VersionInfo,
	side: Side,
	mode: Mode,
	forge_version: &str,
	inst_dir: PathBuf,
	jvm_path: &Path,
	game_jar_path: &Path,
	o: &mut impl NitroOutput,
) -> anyhow::Result<(Classpath, String, (Vec<String>, Vec<String>))> {
	let forge_dir = internal_dir.join("forge");

	let installer_path = forge_dir.join(format!("{}-{forge_version}-installer.jar", mode.to_str()));
	create_leading_dirs(&installer_path)?;

	if !installer_path.exists() || update_depth == UpdateDepth::Force {
		let mut process = o.get_process();
		process.display(
			MessageContents::StartProcess(format!("Downloading {mode} installer")),
			MessageLevel::Important,
		);

		match mode {
			Mode::NeoForge => neoforge::download_installer(forge_version, &installer_path, client)
				.await
				.context("Failed to download installer")?,
		}
		process.display(
			MessageContents::Success(format!("{mode} installer downloaded")),
			MessageLevel::Important,
		);
	}

	// let result = run_installer(&installer_path, side, inst_dir, jvm_path);
	// if let Err(e) = result {
	// 	// Prevents skipping the download
	// 	let _ = std::fs::remove_file(installer_path);
	// 	bail!("Failed to run installer: {e}");
	// }

	let data = extract_installer_data(&installer_path, side)
		.context("Failed to extract installer data")?;

	let mut process = o.get_process();
	process.display(
		MessageContents::StartProcess(format!("Installing {mode} libraries")),
		MessageLevel::Important,
	);

	libraries::get(
		&data.version_json.libraries,
		internal_dir,
		&version_info.version,
		&UpdateManager::new(update_depth),
		client,
		process.deref_mut(),
	)
	.await
	.context("Failed to get libraries")?;

	let classpath = libraries::get_classpath(&data.version_json.libraries, internal_dir)?;

	// Handle arguments
	let (jvm_args, game_args) = match side {
		Side::Client => {
			let Arguments::New(args) = data.version_json.arguments else {
				bail!("Invalid arguments");
			};

			let mut jvm_args = Vec::new();
			let mut game_args = Vec::new();

			let libs_dir = internal_dir.join("libraries").to_string_lossy().to_string();
			let classpath_sep = CLASSPATH_SEP.to_string();

			for arg in args.jvm {
				if let ArgumentItem::Simple(arg) = arg {
					let arg = arg.replace("${version_name}", &version_info.version);
					let arg = arg.replace("${library_directory}", &libs_dir);
					let arg = arg.replace("${classpath_separator}", &classpath_sep);
					jvm_args.push(arg);
				}
			}

			for arg in args.game {
				if let ArgumentItem::Simple(arg) = arg {
					game_args.push(arg);
				}
			}

			(jvm_args, game_args)
		}
		Side::Server => (data.server_args, vec![]),
	};

	// Run setup tasks
	run_processors(
		&data.launcher_profile,
		side,
		jvm_path,
		game_jar_path,
		internal_dir,
	)
	.context("Failed to run processsing tasks")?;

	process.display(
		MessageContents::Success(format!("{mode} installed")),
		MessageLevel::Important,
	);

	Ok((
		classpath,
		data.version_json.main_class,
		(jvm_args, game_args),
	))
}

/// Runs the installer at the given path
fn run_installer(
	path: &Path,
	side: Side,
	inst_dir: PathBuf,
	jvm_path: &Path,
) -> anyhow::Result<()> {
	let mut command = Command::new(jvm_path);

	command.arg("-jar");
	command.arg(path);

	match side {
		Side::Client => command.arg("--installClient"),
		Side::Server => command.arg("--installServer"),
	};

	command.arg(inst_dir);

	let exit = command.spawn()?.wait()?;
	if !exit.success() {
		bail!("Installer returned non-zero status: {exit}")
	}

	Ok(())
}

/// Extracts Forge data from the installer JAR
fn extract_installer_data(installer_path: &Path, side: Side) -> anyhow::Result<InstallerData> {
	let mut zip =
		ZipArchive::new(File::open(installer_path)?).context("Failed to open zip archive")?;

	// Client meta
	let version_json: VersionJson = serde_json::from_reader(BufReader::new(
		zip.by_name("version.json")
			.context("Failed to get version file")?,
	))
	.context("Failed to deserialize version information")?;

	// Launcher profile
	let launcher_profile: LauncherProfile = serde_json::from_reader(BufReader::new(
		zip.by_name("install_profile.json")
			.context("Failed to get version file")?,
	))
	.context("Failed to deserialize launcher profile")?;

	// Arguments for the server
	let server_args = if let Side::Server = side {
		#[cfg(target_os = "windows")]
		let args_path = "data/win_args.txt";
		#[cfg(not(target_os = "windows"))]
		let args_path = "data/unix_args.txt";

		let contents = zip
			.by_name(args_path)
			.context("Failed to get args file from zip")?;

		let result: std::io::Result<Vec<_>> = BufReader::new(contents).lines().collect();
		let result = result?;

		// The text files sometimes put multiple args on the same line
		let mut out = Vec::new();
		for line in result {
			out.extend(line.split(" ").map(|x| x.to_string()));
		}

		out.into_iter().collect()
	} else {
		vec![]
	};

	Ok(InstallerData {
		version_json,
		launcher_profile,
		server_args,
	})
}

/// Data extracted from the installer JAR
struct InstallerData {
	version_json: VersionJson,
	launcher_profile: LauncherProfile,
	server_args: Vec<String>,
}

/// Runs processors from the launcher profile
fn run_processors(
	launcher_profile: &LauncherProfile,
	side: Side,
	jvm_path: &Path,
	game_jar_path: &Path,
	internal_dir: &Path,
) -> anyhow::Result<()> {
	let mut children = Vec::new();

	let game_jar_path = game_jar_path.to_string_lossy().to_string();
	let internal_dir_string = internal_dir.to_string_lossy().to_string();

	let libraries_dir = internal_dir.join("libraries");

	for processor in &launcher_profile.processors {
		if let Some(sides) = &processor.sides {
			if !sides.contains(&side) {
				continue;
			}
		}

		// Skip the EXTRACT_FILES task
		if processor.args.contains(&"EXTRACT_FILES".into()) {
			continue;
		}

		let args = processor.args.iter().map(|arg| {
			let arg = arg.replace("{ROOT}", &internal_dir_string);
			let arg = arg.replace("{SIDE}", &side.to_string());
			let mut arg = arg.replace("{MINECRAFT_JAR}", &game_jar_path);

			for (data_key, data_entry) in &launcher_profile.data {
				let value = match side {
					Side::Client => &data_entry.client,
					Side::Server => &data_entry.server,
				};
				arg = arg.replace(&format!("{{{data_key}}}"), value);
			}

			arg
		});

		let mut command = Command::new(jvm_path);

		let mut classpath = Classpath::new();
		classpath.add_multiple_paths(processor.classpath.iter().filter_map(|x| {
			MavenLibraryParts::parse_from_str(x).map(|x| libraries_dir.join(x.get_dir()))
		}));
		command.env("CLASSPATH", classpath.get_str());

		command.arg("-jar");
		let maven = MavenLibraryParts::parse_from_str(&processor.jar)
			.context("Task JAR was in incorrect format")?;
		command.arg(libraries_dir.join(maven.get_dir()));

		command.args(args);

		eprintln!("{command:?}");

		children.push(command.spawn()?);
	}

	let mut results = Vec::new();

	while !children.is_empty() {
		children.retain_mut(|child| {
			let result = child.try_wait();

			match result {
				Ok(Some(result)) => {
					if !result.success() {
						results.push(anyhow!("Process returned non-zero exit status: {result}"));
					}

					false
				}
				Ok(None) => true,
				Err(e) => {
					results.push(anyhow!("Process failed to run: {e}"));

					false
				}
			}
		});
	}

	for result in results {
		return Err(result);
	}

	Ok(())
}

/// Limited version of the client meta used inside of the Forge installer
#[derive(Deserialize)]
struct VersionJson {
	/// Arguments for the client. Can have a different field name and format
	/// depending on how new the file is in the manifest
	#[serde(alias = "minecraftArguments")]
	arguments: Arguments,
	/// Libraries to download for the client
	libraries: Vec<nitro_core::net::game_files::client_meta::libraries::Library>,
	/// Java main class for the client
	#[serde(rename = "mainClass")]
	main_class: String,
}

/// Mojang launcher profile that we need install tasks from
#[derive(Deserialize)]
struct LauncherProfile {
	/// Side-specific data inserted into task arguments
	data: HashMap<String, DataEntry>,
	/// Tasks to complete
	processors: Vec<Processor>,
}

/// Entry for multiple-sided data
#[derive(Deserialize)]
struct DataEntry {
	client: String,
	server: String,
}

/// A single task to do
#[derive(Deserialize)]
struct Processor {
	#[serde(default)]
	sides: Option<Vec<Side>>,
	jar: String,
	classpath: Vec<String>,
	args: Vec<String>,
}
