/// System Java installation
mod system;

use std::env::consts::EXE_SUFFIX;
use std::fs::File;
use std::io::{BufReader, Read, Seek};

#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context};
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::{translate, UpdateDepth};
use tar::Archive;
use zip::ZipArchive;

use crate::io::files::{self, paths::Paths};
use crate::io::persistent::PersistentData;
use crate::io::update::UpdateManager;
use crate::net::{self, download};
use nitro_shared::util::preferred_archive_extension;

use super::JavaMajorVersion;

/// Type of Java installation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum JavaInstallationKind {
	/// Automatically chooses different Java
	/// flavors based on system conditions
	Auto,
	/// Trys to use a Java installation that is
	/// already on the system
	System,
	/// Adoptium
	Adoptium,
	/// A user-specified installation
	Custom(String),
}

impl JavaInstallationKind {
	/// Parse a string into a JavaKind
	pub fn parse(string: &str) -> Self {
		match string {
			"auto" => Self::Auto,
			"system" => Self::System,
			"adoptium" => Self::Adoptium,
			path => Self::Custom(path.to_string()),
		}
	}
}

/// A Java installation used to launch the game
#[derive(Debug, Clone)]
pub struct JavaInstallation {
	/// The major version of the Java installation
	major_version: JavaMajorVersion,
	/// The path to the directory where the installation is, which will be filled when it is installed
	path: PathBuf,
}

impl JavaInstallation {
	/// Load a new Java installation
	pub(crate) async fn install(
		kind: JavaInstallationKind,
		major_version: JavaMajorVersion,
		mut params: JavaInstallParameters<'_>,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Self> {
		o.start_process();
		o.display(
			MessageContents::StartProcess(translate!(o, StartCheckingForJavaUpdates)),
			MessageLevel::Important,
		);

		let vers_str = major_version.to_string();

		let path = match &kind {
			JavaInstallationKind::Auto => install_auto(&vers_str, params, o).await?,
			JavaInstallationKind::System => system::install(&vers_str)?,
			JavaInstallationKind::Adoptium => install_adoptium(&vers_str, &mut params, o).await?,
			JavaInstallationKind::Custom(id) => {
				// Check if we need to update the install
				let existing_dir = get_existing_dir(
					id,
					&vers_str,
					&params.persistent,
					params.update_manager.get_depth(),
				);

				if let Some(existing_dir) = existing_dir {
					o.display(
						MessageContents::Simple(format!("Using existing Java installation")),
						MessageLevel::Debug,
					);
					existing_dir
				} else {
					if params.custom_install_func.is_some() {
						println!("Install func available");
					}
					// Check if the custom function handles this installation. If it doesn't, assume it's a custom path instead.
					if let Some(func) = params.custom_install_func {
						let result = func
							.install(id, &vers_str, params.update_manager.update_depth)
							.await
							.context("Custom Java install failed")?;

						if let Some(result) = result {
							// Save the version in the persistence file
							params
								.persistent
								.update_java_installation(
									id,
									&vers_str,
									&result.version,
									&result.path,
								)
								.context("Failed to update persistent Java version")?;
							params.persistent.dump(params.paths).await?;

							result.path
						} else {
							PathBuf::from(id)
						}
					} else {
						PathBuf::from(id)
					}
				}
			}
		};

		o.display(
			MessageContents::Success(translate!(o, FinishCheckingForJavaUpdates)),
			MessageLevel::Important,
		);

		o.end_process();

		let out = Self {
			major_version,
			path,
		};

		Ok(out)
	}

	/// Get the major version of the Java installation
	pub fn get_major_version(&self) -> &JavaMajorVersion {
		&self.major_version
	}

	/// Get the path to the Java installation
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Get the path to the JVM.
	pub fn get_jvm_path(&self) -> PathBuf {
		let filename = format!("java{}", EXE_SUFFIX);

		// In case it was set as the JVM path
		if self
			.path
			.file_name()
			.is_some_and(|x| x.to_string_lossy() == filename)
		{
			return self.path.clone();
		}

		let bin_path = self.path.join("bin").join(&filename);
		if bin_path.exists() {
			bin_path
		} else {
			self.path.join(filename)
		}
	}

	/// Verifies that this installation is set up correctly
	pub fn verify(&self) -> anyhow::Result<()> {
		let jvm_path = self.get_jvm_path();
		if !jvm_path.exists() || jvm_path.is_dir() {
			bail!(
				"Java executable (path {:?}) does not exist or is a folder",
				jvm_path
			);
		}
		#[cfg(target_family = "unix")]
		{
			// Check if JVM is executable
			let mut permissions = jvm_path
				.metadata()
				.context("Failed to get JVM metadata")?
				.permissions();
			if permissions.mode() & 0o111 == 0 {
				permissions.set_mode(0o775);
				std::fs::set_permissions(jvm_path, permissions)
					.context("Failed to make Java executable")?;
			}
		}

		Ok(())
	}
}

/// Container struct for parameters for loading Java installations
pub(crate) struct JavaInstallParameters<'a> {
	pub paths: &'a Paths,
	pub update_manager: &'a mut UpdateManager,
	pub persistent: &'a mut PersistentData,
	pub req_client: &'a reqwest::Client,
	pub custom_install_func: Option<&'a Arc<dyn CustomJavaFunction>>,
}

/// Gets the existing dir of a Java installation from the persistent file
fn get_existing_dir(
	java: &str,
	major_version: &str,
	persistent: &PersistentData,
	update_depth: UpdateDepth,
) -> Option<PathBuf> {
	if update_depth != UpdateDepth::Shallow {
		return None;
	}

	let directory = persistent.get_java_path(java, major_version)?;
	if directory.exists() {
		None
	} else {
		Some(directory)
	}
}

async fn install_auto(
	major_version: &str,
	mut params: JavaInstallParameters<'_>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<PathBuf> {
	let out = system::install(major_version);
	if let Ok(out) = out {
		return Ok(out);
	}
	let out = install_adoptium(major_version, &mut params, o).await;
	if let Ok(out) = out {
		return Ok(out);
	}
	bail!("Failed to automatically install Java")
}

async fn install_adoptium(
	major_version: &str,
	params: &mut JavaInstallParameters<'_>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<PathBuf> {
	if params.update_manager.update_depth == UpdateDepth::Shallow {
		if let Some(directory) = params.persistent.get_java_path("adoptium", major_version) {
			if directory.exists() {
				Ok(directory)
			} else {
				update_adoptium(major_version, params, o)
					.await
					.context("Failed to update Adoptium Java")
			}
		} else {
			update_adoptium(major_version, params, o)
				.await
				.context("Failed to update Adoptium Java")
		}
	} else {
		update_adoptium(major_version, params, o)
			.await
			.context("Failed to update Adoptium Java")
	}
}

/// Updates Adoptium and returns the path to the installation
async fn update_adoptium(
	major_version: &str,
	params: &mut JavaInstallParameters<'_>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<PathBuf> {
	let out_dir = params.paths.java.join("adoptium");
	files::create_dir(&out_dir)?;
	let version = net::java::adoptium::get_latest(major_version, params.req_client)
		.await
		.context("Failed to obtain Adoptium information")?;

	let release_name = version.release_name.clone();
	let mut extracted_bin_name = release_name.clone();
	extracted_bin_name.push_str("-jre");
	let extracted_bin_dir = out_dir.join(&extracted_bin_name);

	if !params
		.persistent
		.update_java_installation("adoptium", major_version, &release_name, &extracted_bin_dir)
		.context("Failed to update Java in lockfile")?
	{
		return Ok(extracted_bin_dir);
	}

	params.persistent.dump(params.paths).await?;

	let arc_extension = preferred_archive_extension();
	let arc_name = format!("adoptium{major_version}{arc_extension}");
	let arc_path = out_dir.join(arc_name);

	let bin_url = version.binary.package.link;

	o.display(
		MessageContents::StartProcess(translate!(
			o,
			DownloadingAdoptium,
			"version" = &release_name
		)),
		MessageLevel::Important,
	);
	download::file(bin_url, &arc_path, params.req_client)
		.await
		.context("Failed to download JRE binaries")?;

	// Extraction
	o.display(
		MessageContents::StartProcess(translate!(o, StartExtractingJava)),
		MessageLevel::Important,
	);
	extract_archive_file(&arc_path, &out_dir).context("Failed to extract")?;
	o.display(
		MessageContents::StartProcess(translate!(o, StartRemovingJavaArchive)),
		MessageLevel::Important,
	);
	std::fs::remove_file(arc_path).context("Failed to remove archive")?;

	o.display(
		MessageContents::Success(translate!(o, FinishJavaInstallation)),
		MessageLevel::Important,
	);

	// MacOS does some screwery
	#[cfg(not(target_os = "macos"))]
	let final_dir = extracted_bin_dir;
	#[cfg(target_os = "macos")]
	let final_dir = extracted_bin_dir.join("Contents/Home");

	Ok(final_dir)
}

/// Function for custom Java handling
#[async_trait::async_trait]
pub trait CustomJavaFunction: Send + Sync {
	/// Call the custom install function
	async fn install(
		&self,
		java: &str,
		major_version: &str,
		update_depth: UpdateDepth,
	) -> anyhow::Result<Option<CustomJavaFunctionResult>>;
}

/// Result from the custom Java function
pub struct CustomJavaFunctionResult {
	/// Path to the new installation
	pub path: PathBuf,
	/// Version of the new installation
	pub version: String,
}

/// Extracts the archive file
fn extract_archive_file(arc_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
	let file = File::open(arc_path).context("Failed to read archive file")?;
	let file = BufReader::new(file);

	extract_archive(file, out_dir)?;

	Ok(())
}

/// Extracts the JRE archive (either a tar or a zip) and also returns the internal extraction directory name
fn extract_archive<R: Read + Seek>(reader: R, out_dir: &Path) -> anyhow::Result<String> {
	let dir_name = if cfg!(windows) {
		let mut archive = ZipArchive::new(reader).context("Failed to open zip archive")?;

		let dir_name = archive
			.file_names()
			.next()
			.context("Missing archive internal directory")?
			.to_string();

		archive
			.extract(out_dir)
			.context("Failed to extract zip file")?;

		dir_name
	} else {
		let mut decoder =
			libflate::gzip::Decoder::new(reader).context("Failed to decode tar.gz")?;
		// Get the archive twice because of archive shenanigans
		let mut arc = Archive::new(&mut decoder);

		// Wow
		let dir_name = arc
			.entries()
			.context("Failed to get Tar entries")?
			.next()
			.context("Missing archive internal directory")?
			.context("Failed to get entry")?
			.path()
			.context("Failed to get entry path name")?
			.to_string_lossy()
			.to_string();

		let mut arc = Archive::new(&mut decoder);
		arc.unpack(out_dir).context("Failed to unarchive tar")?;

		dir_name
	};

	Ok(dir_name)
}
