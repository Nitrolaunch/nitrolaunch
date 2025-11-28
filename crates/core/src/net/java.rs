use crate::net::download;
use nitro_shared::util::{ARCH_STRING, OS_STRING};

use anyhow::Context;
use reqwest::Client;

/// Downloading Adoptium JDK
pub mod adoptium {
	use anyhow::bail;
	use serde::Deserialize;

	use super::*;

	/// Gets the newest Adoptium binaries download for a major Java version
	pub async fn get_latest(major_version: &str, client: &Client) -> anyhow::Result<PackageFormat> {
		let url = json_url(major_version);
		let mut manifest = download::json::<Vec<PackageFormat>>(&url, client)
			.await
			.context("Failed to download manifest of Adoptium versions")?;
		if manifest.is_empty() {
			bail!("A valid installation was not found");
		}
		let version = manifest.swap_remove(0);

		Ok(version)
	}

	/// Gets the URL to the JSON file for a major Java version
	fn json_url(major_version: &str) -> String {
		format!(
			"https://api.adoptium.net/v3/assets/latest/{major_version}/hotspot?image_type=jre&vendor=eclipse&architecture={}&os={}",
			get_arch_arg(),
			get_os_arg(),
		)
	}

	/// Get the OS argument for the API
	fn get_os_arg() -> &'static str {
		if cfg!(target_os = "macos") {
			"mac"
		} else {
			OS_STRING
		}
	}

	/// Get the arch argument for the API
	fn get_arch_arg() -> &'static str {
		if cfg!(target_arch = "x86_64") {
			"x64"
		} else {
			ARCH_STRING
		}
	}

	/// A single package info for Adoptium
	#[derive(Deserialize, Debug)]
	pub struct PackageFormat {
		/// Information about the binary
		pub binary: Binary,
		/// Name of the Java release
		pub release_name: String,
	}

	/// Binary for an Adoptium package
	#[derive(Deserialize, Debug)]
	pub struct Binary {
		/// Package field that contains the download link
		pub package: BinaryPackage,
	}

	/// Package field inside the binary struct
	#[derive(Deserialize, Debug)]
	pub struct BinaryPackage {
		/// Link to the JRE download
		pub link: String,
	}
}
