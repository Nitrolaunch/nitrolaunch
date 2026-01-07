use std::{fmt::Display, ops::DerefMut, path::Path, process::Command};

use anyhow::{bail, Context};
use nitro_core::{
	io::{
		files::create_leading_dirs,
		java::classpath::{Classpath, CLASSPATH_SEP},
		json_from_file,
	},
	net::game_files::{
		client_meta::{
			args::{ArgumentItem, Arguments},
			ClientMeta,
		},
		libraries::get_classpath,
	},
};
use nitro_net::neoforge;
use nitro_shared::{
	no_window,
	output::{MessageContents, MessageLevel, NitroOutput},
	versions::VersionInfo,
	Side, UpdateDepth,
};
use reqwest::Client;

/// Mode we are in (Forge / NeoForge)
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

/// Installs Forge or NeoForge from a version. Returns the classpath, main class, and JVM and game args
pub async fn install(
	client: &Client,
	internal_dir: &Path,
	update_depth: UpdateDepth,
	version_info: &VersionInfo,
	side: Side,
	mode: Mode,
	forge_version: &str,
	jvm_path: &Path,
	o: &mut impl NitroOutput,
) -> anyhow::Result<ForgeInstallResult> {
	let forge_dir = internal_dir.join("forge").join(mode.to_str());

	let installer_file_name = format!("{}-{forge_version}-installer.jar", mode.to_str());
	let installer_path = forge_dir.join(&installer_file_name);
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

	if side == Side::Client {
		create_mojang_launcher_jsons(internal_dir)
			.context("Failed to create Microsoft launcher JSONs")?;
	}

	let client_meta_path = internal_dir
		.join("versions")
		.join(format!("{}-{forge_version}", mode.to_str()))
		.join(format!("{}-{forge_version}.json", mode.to_str()));

	let server_jar_path = match mode {
		Mode::NeoForge => internal_dir
			.join("libraries")
			.join("net/neoforged/neoforge/{forge_version}/neoforge-{forge_version}-server.jar"),
	};

	let already_installed = match side {
		Side::Client => client_meta_path.exists(),
		Side::Server => server_jar_path.exists(),
	};
	let already_installed = already_installed || update_depth == UpdateDepth::Force;

	let mut process = o.get_process();
	process.display(
		MessageContents::StartProcess(format!("Checking {mode} version info")),
		MessageLevel::Important,
	);

	// Run the installer if not not installed
	if !already_installed {
		process.display(
			MessageContents::StartProcess(format!("Running {mode} installer")),
			MessageLevel::Important,
		);

		let result = run_installer(
			&installer_path,
			side,
			internal_dir,
			&forge_dir,
			jvm_path,
			process.deref_mut(),
		);
		if let Err(e) = result {
			let log_file_name = installer_file_name + ".log";
			if let Ok(log_text) = std::fs::read_to_string(forge_dir.join(log_file_name)) {
				process.display(
					MessageContents::Error(format!("Installer log:\n{log_text}")),
					MessageLevel::Important,
				);
			}
			bail!("Failed to run installer: {e}");
		}

		process.display(
			MessageContents::Success(format!("{mode} installer ran")),
			MessageLevel::Important,
		);
	}

	// Get data based on the side

	match side {
		Side::Client => {
			let client_meta: ClientMeta = json_from_file(&client_meta_path)
				.context("Failed to read version JSON for Forge")?;

			let Arguments::New(args) = client_meta.arguments else {
				bail!("Arguments in incorrect format");
			};

			let libraries_dir = internal_dir.join("libraries");

			let jvm_args = args
				.jvm
				.into_iter()
				.filter_map(|x| {
					if let ArgumentItem::Simple(arg) = x {
						Some(process_arg(&arg, &libraries_dir, &version_info.version))
					} else {
						None
					}
				})
				.collect();

			let game_args = args
				.game
				.into_iter()
				.filter_map(|x| {
					if let ArgumentItem::Simple(arg) = x {
						Some(process_arg(&arg, &libraries_dir, &version_info.version))
					} else {
						None
					}
				})
				.collect();

			let classpath = get_classpath(&client_meta.libraries, internal_dir)
				.context("Failed to get classpath")?;

			process.display(
				MessageContents::Success(format!("{mode} installed")),
				MessageLevel::Important,
			);

			Ok(ForgeInstallResult {
				classpath,
				main_class: client_meta.main_class,
				jvm_args,
				game_args,
				exclude_game_jar: true,
			})
		}
		Side::Server => {
			// libraries/net/neoforged/neoforge/20.2.93/unix_args.txt
			bail!("Forge server is not currently supported");
		}
	}
}

/// Result from the Forge install() function, to be added to the defaults from the client meta.
pub struct ForgeInstallResult {
	/// Java classpath
	pub classpath: Classpath,
	/// Java main class
	pub main_class: String,
	/// Args for the JVM
	pub jvm_args: Vec<String>,
	/// Args for the game
	pub game_args: Vec<String>,
	/// Whether to skip adding the game JAR to the final classpath
	pub exclude_game_jar: bool,
}

/// Runs the installer at the given path
fn run_installer(
	path: &Path,
	side: Side,
	install_dir: &Path,
	forge_dir: &Path,
	jvm_path: &Path,
	o: &mut impl NitroOutput,
) -> anyhow::Result<()> {
	let mut command = Command::new(jvm_path);

	command.arg("-jar");
	command.arg(path);

	match side {
		Side::Client => command.arg("--installClient"),
		Side::Server => command.arg("--installServer"),
	};

	command.arg(install_dir);

	// This is where the log file will end up
	command.current_dir(forge_dir);

	no_window!(command);

	o.display(
		MessageContents::Simple(format!("{command:?}")),
		MessageLevel::Debug,
	);

	let exit = command.spawn()?.wait()?;
	if !exit.success() {
		bail!("Installer returned non-zero status: {exit}")
	}

	Ok(())
}

/// Creates JSON files required for the installer to work
fn create_mojang_launcher_jsons(dir: &Path) -> anyhow::Result<()> {
	std::fs::write(dir.join("launcher_profiles.json"), "{}")?;
	std::fs::write(dir.join("launcher_profiles_microsoft_store.json"), "{}")?;
	Ok(())
}

/// Processes an argument from the version JSON to replace tokens
fn process_arg(arg: &str, libraries_dir: &Path, version_name: &str) -> String {
	let arg = arg.replace("${classpath_separator}", &format!("{CLASSPATH_SEP}"));
	let arg = arg.replace("${library_directory}", &*libraries_dir.to_string_lossy());
	let arg = arg.replace("${version_name}", version_name);

	arg
}
