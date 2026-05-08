use std::{
	io::Write,
	path::{Path, PathBuf},
};

use nitro_plugin::api::wasm::sys::{get_data_dir, run_command};
use sha2::Digest;

/// Version number for stored files
static VERSION: &str = "v1";

/// Hashes a classpath with sha256
pub fn hash_classpath(classpath: &str) -> String {
	let mut hash = sha2::Sha256::new();
	let _ = hash.write(classpath.as_bytes());
	let hash = hash.finalize();
	hex::encode(hash)
}

fn get_cds_dir() -> PathBuf {
	get_data_dir().join(format!("internal/cds/{VERSION}"))
}

/// Gets the cached class list and dump for the given classpath hash
pub fn get_cached_paths(hash: &str) -> (PathBuf, PathBuf) {
	let base_dir = get_cds_dir();
	(
		base_dir.join(format!("{hash}.lst")),
		base_dir.join(format!("{hash}.jsa")),
	)
}

/// Gets JVM arguments for class list creation on the first launch
pub fn get_list_creation_args(list_path: &Path) -> Vec<String> {
	vec![
		"-Xshare:off".into(),
		format!("-XX:DumpLoadedClassList={}", list_path.to_string_lossy()),
	]
}

/// Gets JVM arguments for using a class dump when it is ready
pub fn get_dump_use_args(dump_path: &Path) -> Vec<String> {
	vec![
		"-Xshare:auto".into(),
		format!("-XX:SharedArchiveFile={}", dump_path.to_string_lossy()),
	]
}

/// Spawns a JVM process to create the dump
pub fn create_dump(
	list_path: &Path,
	dump_path: &Path,
	classpath: String,
	jvm_path: &Path,
) -> anyhow::Result<()> {
	// Copy list file
	let list_filename = list_path.file_name().unwrap();
	let list_copy = list_path.with_file_name(format!("{}2", list_filename.to_string_lossy()));
	std::fs::copy(list_path, &list_copy)?;

	let args = vec![
		"-Xshare:dump".into(),
		format!("-XX:SharedClassListFile={}", list_copy.to_string_lossy()),
		format!("-XX:SharedArchiveFile={}", dump_path.to_string_lossy()),
		"-cp".into(),
		classpath,
	];

	let stdout = get_cds_dir().join("log");

	run_command(
		jvm_path,
		args,
		None::<&str>,
		Some(stdout),
		true,
		true,
		false,
	)?;

	Ok(())
}
