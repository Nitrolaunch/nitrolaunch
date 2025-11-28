use std::{
	fs::File,
	io::{BufReader, Read, Seek},
	path::Path,
};

use anyhow::Context;
use nitro_plugin::{
	api::wasm::{
		net::{download_bytes, download_file},
		sys::{get_arch_string, get_data_dir, get_os_string},
		WASMPlugin,
	},
	hook::hooks::InstallCustomJavaResult,
	nitro_wasm_plugin,
};
use serde::Deserialize;
use tar::Archive;
use zip::ZipArchive;

nitro_wasm_plugin!(main, "zulu");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.install_custom_java(|arg| {
		if arg.kind != "zulu" {
			return Ok(None);
		}

		let out_dir = get_data_dir().join("internal/java/zulu");
		if !out_dir.exists() {
			let _ = std::fs::create_dir_all(&out_dir);
		}

		let package =
			get_latest(&arg.major_version).context("Failed to get the latest Zulu version")?;

		let version = extract_dir_name(&package.name);

		let extracted_dir = out_dir.join(&version);

		let arc_path = out_dir.join(&package.name);

		download_file(package.download_url, &arc_path)
			.context("Failed to download JRE binaries")?;

		// Extraction
		extract_archive_file(&arc_path, &out_dir).context("Failed to extract")?;
		std::fs::remove_file(arc_path).context("Failed to remove archive")?;

		Ok(Some(InstallCustomJavaResult {
			path: extracted_dir.to_string_lossy().to_string(),
			version: version.replace("zulu", ""),
		}))
	})?;

	Ok(())
}

/// Gets the newest Zulu package for a major Java version
fn get_latest(major_version: &str) -> anyhow::Result<PackageFormat> {
	let url = json_url(major_version);
	let bytes = download_bytes(url).context("Failed to download manifest of Zulu versions")?;
	let manifest: Vec<PackageFormat> = serde_json::from_slice(&bytes)
		.context("Failed to deserialize manifest of Zulu versions")?;
	let package = manifest
		.first()
		.context("A valid installation was not found")?;

	Ok(package.to_owned())
}

/// Gets the URL to the JSON file for a major Java version
fn json_url(major_version: &str) -> String {
	let os = get_os_string();
	let arch = get_arch_string();
	let preferred_archive = get_preferred_archive();
	format!(
			"https://api.azul.com/metadata/v1/zulu/packages/?java_version={major_version}&os={os}&arch={arch}&archive_type={preferred_archive}&java_package_type=jre&latest=true&java_package_features=headfull&release_status=ga&availability_types=CA&certifications=tck&page=1&page_size=100"
		)
}

/// Format of the metadata JSON with download info for Zulu
#[derive(Deserialize, Clone)]
pub struct PackageFormat {
	/// Name of the Zulu version
	pub name: String,
	/// Download URL for the package
	pub download_url: String,
}

/// Gets the name of the extracted directory by removing the archive file extension
fn extract_dir_name(name: &str) -> String {
	let archive_extension = format!(".{}", get_preferred_archive());

	name.replacen(&archive_extension, "", 1)
}

/// Gets the preferred archive
fn get_preferred_archive() -> &'static str {
	match get_os_string().as_str() {
		"linux" => "tar.gz",
		_ => "zip",
	}
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
	let dir_name = if get_os_string() == "windows" {
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
		// Manual extraction implementation since WASI-p2 doesn't support fs::canonicalize
		for entry in arc.entries()? {
			let mut entry = entry?;
			let dest_path = out_dir.join(entry.path()?);
			if dest_path.to_string_lossy().ends_with("/") {
				if !dest_path.exists() {
					let _ = std::fs::create_dir(dest_path);
				}
				continue;
			}

			if let Some(parent) = dest_path.parent() {
				if !parent.exists() {
					std::fs::create_dir_all(parent)?;
				}
			}

			let mut out_file =
				File::create(dest_path).context("Failed to open destination file")?;
			std::io::copy(&mut entry, &mut out_file).context("Failed to copy file")?;
		}

		dir_name
	};

	Ok(dir_name)
}
