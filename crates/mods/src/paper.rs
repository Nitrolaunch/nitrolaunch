use std::{fmt::Display, path::PathBuf};

use anyhow::{anyhow, bail, Context};
use nitro_core::{net::download, NitroCore};
use nitro_shared::{output::NitroOutput, versions::VersionInfo, Side};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use nitro_core::io::files::paths::Paths;

/// The main class for a Paper/Folia server
pub const PAPER_SERVER_MAIN_CLASS: &str = "io.papermc.paperclip.Main";

/// The main class for the Velocity proxy
pub const VELOCITY_MAIN_CLASS: &str = "com.velocitypowered.proxy.Velocity";

/// Different modes for this module, depending on which project you want to install
#[derive(Debug, Clone, Copy)]
pub enum Mode {
	/// The Paper server
	Paper,
	/// The Folia multithreaded server
	Folia,
	/// The Velocity proxy
	Velocity,
}

impl Mode {
	/// Convert this mode to a lowercase string
	pub fn to_str(self) -> &'static str {
		match self {
			Self::Paper => "paper",
			Self::Folia => "folia",
			Self::Velocity => "velocity",
		}
	}
}

impl Display for Mode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Paper => write!(f, "Paper"),
			Self::Folia => write!(f, "Folia"),
			Self::Velocity => write!(f, "Velocity"),
		}
	}
}

/// Install Paper or Folia using the core and information about the version.
/// This function will throw an error if Velocity is passed as a mode.
/// First, create the core and the version you want. Then, get the version info from the version.
/// Finally, run this function. Returns the JAR path and main class to add to the instance you are launching
pub async fn install_from_core(
	core: &mut NitroCore,
	version_info: &VersionInfo,
	mode: Mode,
	o: &mut impl NitroOutput,
) -> anyhow::Result<(PathBuf, String)> {
	let _ = o;

	if let Mode::Velocity = mode {
		bail!("Velocity is a proxy and cannot be used in the install_from_core function");
	}

	let build_num = get_newest_build(mode, &version_info.version, core.get_client())
		.await
		.context(format!("Failed to get newest {mode} build"))?;
	let jar_file_name =
		get_jar_file_name(mode, &version_info.version, build_num, core.get_client())
			.await
			.context(format!("Failed to get the API name of the {mode} JAR file"))?;
	download_server_jar(
		mode,
		&version_info.version,
		build_num,
		&jar_file_name,
		core.get_paths(),
		core.get_client(),
	)
	.await
	.context(format!("Failed to download {mode} JAR file"))?;

	Ok((
		get_local_jar_path(mode, &version_info.version, core.get_paths()),
		PAPER_SERVER_MAIN_CLASS.into(),
	))
}

/// Install Velocity, returning the path to the JAR file and the main class
pub async fn install_velocity(paths: &Paths, client: &Client) -> anyhow::Result<(PathBuf, String)> {
	let version = get_newest_version(Mode::Velocity, client)
		.await
		.context("Failed to get newest Velocity version")?;
	let build_num = get_newest_build(Mode::Velocity, &version, client)
		.await
		.context("Failed to get newest Velocity build version")?;
	let file_name = get_jar_file_name(Mode::Velocity, &version, build_num, client)
		.await
		.context("Failed to get Velocity build file name")?;

	download_server_jar(
		Mode::Velocity,
		&version,
		build_num,
		&file_name,
		paths,
		client,
	)
	.await
	.context("Failed to download Velocity JAR")?;

	Ok((
		get_local_jar_path(Mode::Velocity, &version, paths),
		VELOCITY_MAIN_CLASS.into(),
	))
}

/// Get all versions of a PaperMC project
pub async fn get_all_versions(mode: Mode, client: &Client) -> anyhow::Result<Vec<String>> {
	let url = format!("https://api.papermc.io/v2/projects/{}", mode.to_str());
	let resp: ProjectInfoResponse = download::json(url, client).await?;
	Ok(resp.versions)
}

/// Get the newest version of a PaperMC project
pub async fn get_newest_version(mode: Mode, client: &Client) -> anyhow::Result<String> {
	let versions = get_all_versions(mode, client).await?;

	let version = versions
		.last()
		.ok_or(anyhow!("Could not find a valid {mode} version"))?;

	Ok(version.clone())
}

#[derive(Deserialize)]
struct ProjectInfoResponse {
	versions: Vec<String>,
}

/// Get all available build numbers of a PaperMC project version
pub async fn get_builds(mode: Mode, version: &str, client: &Client) -> anyhow::Result<Vec<u16>> {
	let url = format!(
		"https://api.papermc.io/v2/projects/{}/versions/{version}",
		mode.to_str(),
	);
	let resp: VersionInfoResponse = download::json(url, client).await?;

	Ok(resp.builds)
}

/// Get the newest build number of a PaperMC project version
pub async fn get_newest_build(mode: Mode, version: &str, client: &Client) -> anyhow::Result<u16> {
	let builds = get_builds(mode, version, client).await?;

	let build = builds
		.iter()
		.max()
		.ok_or(anyhow!("Could not find a valid {mode} build version"))?;

	Ok(*build)
}

/// Info about a project version
#[derive(Serialize, Deserialize)]
pub struct VersionInfoResponse {
	/// The list of available build numbers
	pub builds: Vec<u16>,
}

/// Gets info from the given build
pub async fn get_build_info(
	mode: Mode,
	version: &str,
	build_num: u16,
	client: &Client,
) -> anyhow::Result<BuildInfoResponse> {
	let num_str = build_num.to_string();
	let url = format!(
		"https://api.papermc.io/v2/projects/{}/versions/{version}/builds/{num_str}",
		mode.to_str(),
	);
	let resp: BuildInfoResponse = download::json(url, client).await?;

	Ok(resp)
}

/// Get the name of the Paper JAR file in the API.
/// This does not represent the name of the file when downloaded
/// as it will be stored in the core JAR location
pub async fn get_jar_file_name(
	mode: Mode,
	version: &str,
	build_num: u16,
	client: &Client,
) -> anyhow::Result<String> {
	let info = get_build_info(mode, version, build_num, client).await?;

	Ok(info.downloads.application.name)
}

/// Response from the build info API
#[derive(Serialize, Deserialize)]
pub struct BuildInfoResponse {
	/// The list of downloads
	pub downloads: BuildInfoDownloads,
}

/// Downloads for a build
#[derive(Serialize, Deserialize)]
pub struct BuildInfoDownloads {
	/// Application info for the download
	pub application: BuildInfoApplication,
}

/// Application info for a build download
#[derive(Serialize, Deserialize)]
pub struct BuildInfoApplication {
	/// The name of the JAR file
	pub name: String,
}

/// Download the server jar
pub async fn download_server_jar(
	mode: Mode,
	version: &str,
	build_num: u16,
	file_name: &str,
	paths: &Paths,
	client: &Client,
) -> anyhow::Result<()> {
	let num_str = build_num.to_string();
	let url = format!("https://api.papermc.io/v2/projects/{}/versions/{version}/builds/{num_str}/downloads/{file_name}", mode.to_str());

	let file_path = get_local_jar_path(mode, version, paths);
	download::file(&url, &file_path, client)
		.await
		.context("Failed to download {mode} JAR")?;

	Ok(())
}

/// Get the path to the stored JAR file
pub fn get_local_jar_path(mode: Mode, version: &str, paths: &Paths) -> PathBuf {
	nitro_core::io::minecraft::game_jar::get_path(Side::Server, version, Some(mode.to_str()), paths)
}
