use std::{
	fs::File,
	io::{Read, Seek},
	path::Path,
};

use anyhow::{bail, Context};
use nitro_plugin::{
	api::wasm::{
		net::download_bytes,
		sys::{get_arch_string, get_data_dir, get_os_string},
		WASMPlugin,
	},
	hook::hooks::InstallCustomJavaResult,
	nitro_wasm_plugin,
};
use tar::Archive;
use zip::ZipArchive;

nitro_wasm_plugin!(main, "graalvm");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.install_custom_java(|arg| {
		if arg.kind != "graalvm" {
			return Ok(None);
		}

		if arg.major_version == "17" {
			bail!("GraalVM for Java version 17 is unsupported");
		}

		let out_dir = get_data_dir().join("internal/java/graalvm");
		if !out_dir.exists() {
			let _ = std::fs::create_dir_all(&out_dir);
		}

		let archive = get_latest(&arg.major_version)
			.context("Failed to download the latest GraalVM version")?;

		let dir_name = extract_archive(std::io::Cursor::new(archive), &out_dir)
			.context("Failed to extract GraalVM archive")?;
		let extracted_dir = out_dir.join(&dir_name);

		let version = dir_name.replace("graalvm-jdk-", "");

		Ok(Some(InstallCustomJavaResult {
			path: extracted_dir.to_string_lossy().to_string(),
			version,
		}))
	})?;

	Ok(())
}

/// Downloads the contents of the GraalVM archive
fn get_latest(major_version: &str) -> anyhow::Result<Vec<u8>> {
	let url = download_url(major_version);
	download_bytes(url)
}

/// Gets the download URL
fn download_url(major_version: &str) -> String {
	format!(
			"https://download.oracle.com/graalvm/{major_version}/latest/graalvm-jdk-{major_version}_{}-{}_bin.{}",
			get_os_string(),
			get_arch_string().replace("x86_64", "x64"),
			get_preferred_archive()
		)
}

/// Gets the preferred archive
fn get_preferred_archive() -> &'static str {
	match get_os_string().as_str() {
		"linux" => "tar.gz",
		_ => "zip",
	}
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
