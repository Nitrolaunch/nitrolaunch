use std::path::{Path, PathBuf};

use anyhow::Context;

/// Installs the system java installation
pub fn install(major_version: &str) -> anyhow::Result<PathBuf> {
	let installation = get_system_java_installation(major_version);
	installation.context("No valid system Java installation found")
}

macro_rules! scan {
	($path:expr, $major_version:expr) => {
		if let Some(path) = scan_dir($path, $major_version) {
			return Some(path);
		}
	};
}

/// Gets the optimal path to a system Java installation
fn get_system_java_installation(#[allow(unused_variables)] major_version: &str) -> Option<PathBuf> {
	// JAVA_HOME
	if let Ok(home) = std::env::var("JAVA_HOME") {
		// This isn't a directory holding Java installations, it IS a Java installation
		if home.contains(major_version) {
			let home = PathBuf::from(home);
			if home.join("bin").exists() {
				return Some(home);
			}
		}
	}

	#[cfg(target_os = "windows")]
	{
		if let Some(path) = scan_windows(major_version) {
			return Some(path);
		}
	}
	#[cfg(target_os = "macos")]
	{
		if let Some(path) = scan_macos(major_version) {
			return Some(path);
		}
	}
	#[cfg(target_os = "linux")]
	{
		if let Some(path) = scan_linux(major_version) {
			return Some(path);
		}
	}

	None
}

/// Scan for Java on Windows
#[cfg(target_os = "windows")]
fn scan_windows(major_version: &str) -> Option<PathBuf> {
	// OpenJDK
	scan!(&PathBuf::from("C:/Program Files/Java"), major_version);

	None
}

/// Scan for Java on MacOS
#[cfg(target_os = "macos")]
fn scan_macos(major_version: &str) -> Option<PathBuf> {
	// Homebrew
	scan!(&PathBuf::from("/opt/homebrew/opt/"), major_version);

	None
}

/// Scan for Java on Linux
#[cfg(target_os = "linux")]
fn scan_linux(major_version: &str) -> Option<PathBuf> {
	// OpenJDK
	scan!(&PathBuf::from("/usr/lib/jvm"), major_version);
	scan!(&PathBuf::from("/usr/lib64/jvm"), major_version);
	scan!(&PathBuf::from("/usr/lib32/jvm"), major_version);
	scan!(&PathBuf::from("/usr/lib/java"), major_version);
	// Oracle RPMs
	scan!(&PathBuf::from("/usr/java"), major_version);
	// Manually installed
	scan!(&PathBuf::from("/opt/jvm"), major_version);
	scan!(&PathBuf::from("/opt/jdk"), major_version);
	scan!(&PathBuf::from("/opt/jdks"), major_version);
	// Flatpak
	scan!(&PathBuf::from("/app/jdk"), major_version);

	if let Ok(home) = std::env::var("HOME") {
		let home = PathBuf::from(home);
		// IntelliJ
		scan!(&home.join(".jdks"), major_version);
		// SDKMan
		scan!(&home.join(".sdkman/candidates/java"), major_version);
		// Gradle
		scan!(&home.join(".gradle/jdks"), major_version);
	}

	None
}

/// Scan a directory for Java installations
fn scan_dir(dir: &Path, major_version: &str) -> Option<PathBuf> {
	let debug = std::env::var("NITRO_JAVA_SCAN_DEBUG").is_ok_and(|x| x == "1");
	if debug {
		println!("Scanning {dir:?}");
		dbg!(&major_version);
	}

	if dir.exists() {
		let read = std::fs::read_dir(dir).ok()?;
		for path in read {
			let Ok(path) = path else { continue };
			if debug {
				println!("{:?}", path.path());
			}
			let name = path.file_name().to_string_lossy().to_string();
			let path = path.path();

			if check_single_dir(&path, name, major_version, debug) {
				return Some(path);
			}
		}
	}

	None
}

fn check_single_dir(path: &Path, filename: String, major_version: &str, debug: bool) -> bool {
	if !path.is_dir() {
		if debug {
			println!("Not directory");
		}
		return false;
	}
	if !(filename.contains("java") || filename.contains("jdk")) {
		if debug {
			println!("Not a Java folder");
		}
		return false;
	}
	if !filename.contains(&format!("-{major_version}")) {
		if debug {
			println!("Does not contain major version");
		}
		return false;
	}

	// Make sure there is a bin directory
	if !path.join("bin").exists() {
		if debug {
			println!("No bin directory found");
		}
		return false;
	}

	true
}
