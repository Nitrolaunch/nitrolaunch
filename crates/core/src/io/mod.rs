use std::ffi::CString;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek};
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::Serialize;
use zip::ZipArchive;

/// Global IO configuration using a file or environment variables
pub mod config;
/// Utilities for dealing with the filesystem
pub mod files;
/// Interaction with some of Java's formats
pub mod java;
/// I/O with Minecraft data formats
pub mod minecraft;
/// Use of a file for persistent data
pub mod persistent;
/// Management of file updates
pub mod update;

/// Reads JSON from a file with a buffer
pub fn json_from_file<D: DeserializeOwned>(path: impl AsRef<Path>) -> anyhow::Result<D> {
	let file = BufReader::new(File::open(path).context("Failed to open file")?);
	Ok(simd_json::from_reader(file)?)
}

/// Writes JSON to a file with a buffer
pub fn json_to_file<S: Serialize>(path: impl AsRef<Path>, data: &S) -> anyhow::Result<()> {
	let file = BufWriter::new(File::create(path).context("Failed to open file")?);
	simd_json::to_writer(file, data).context("Failed to serialize data to file")?;
	Ok(())
}

/// Writes JSON to a file with a buffer and pretty formatting
pub fn json_to_file_pretty<S: Serialize>(path: impl AsRef<Path>, data: &S) -> anyhow::Result<()> {
	let file = BufWriter::new(File::create(path).context("Failed to open file")?);
	serde_json::to_writer_pretty(file, data).context("Failed to serialize data to file")?;
	Ok(())
}

/// Writes JSON to a file with less than ideal formatting, but at a higher speed
pub fn json_to_file_pretty_fast<S: Serialize>(
	path: impl AsRef<Path>,
	data: &S,
) -> anyhow::Result<()> {
	let file = BufWriter::new(File::create(path).context("Failed to open file")?);
	simd_json::to_writer_pretty(file, data).context("Failed to serialize data to file")?;
	Ok(())
}

/// Extracts a specific directory within a zip file
pub fn extract_zip_dir<R: Read + Seek>(
	zip: &mut ZipArchive<R>,
	zip_dir: &str,
	target_dir: impl AsRef<Path>,
) -> anyhow::Result<()> {
	let _ = std::fs::create_dir_all(target_dir.as_ref());

	for index in 0..zip.len() {
		let mut file = zip.by_index(index)?;
		if file.is_dir() {
			continue;
		}

		let Some(filename) = file.enclosed_name() else {
			continue;
		};

		let Ok(filename) = filename.strip_prefix(zip_dir) else {
			continue;
		};

		let out_path = target_dir.as_ref().join(filename);
		let _ = files::create_leading_dirs(&out_path);

		let mut out_file = File::create(out_path).context("Failed to create output file")?;

		std::io::copy(&mut file, &mut out_file).context("Failed to copy file from zip")?;
	}

	Ok(())
}

/// Tries to get the user's home dir
pub fn home_dir() -> anyhow::Result<PathBuf> {
	#[cfg(target_os = "linux")]
	let path = std::env::var("HOME")?;
	#[cfg(target_os = "windows")]
	let path = format!("{}/..", std::env::var("%APPDATA%")?);
	#[cfg(target_os = "macos")]
	let path = std::env::var("HOME")?;

	Ok(PathBuf::from(path))
}

#[cfg(target_family = "unix")]
extern "C" {
	fn mkfifo(path: *const i8, mode: u32) -> i32;

	fn open(path: *const i8, flags: i32, mode: u32) -> i32;

	fn fcntl(file: i32, op: i32, mode: i32) -> i32;
}

#[cfg(target_os = "windows")]
use std::os::windows::prelude::RawHandle;

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
	fn CreateNamedPipeA(
		lpName: *const i8,
		dwOpenMode: u32,
		dwPipeMode: u32,
		dwMaxInstances: u32,
		dwOutBufferSize: u32,
		dwInBufferSize: u32,
		dwDefaultTimeOut: u32,
		lpSecurityAttributes: *mut std::ffi::c_void,
	) -> RawHandle;

	fn ConnectNamedPipe(hNamedPipe: RawHandle, lpOverlapped: *mut std::ffi::c_void) -> i32;
}

/// Creates a named pipe at the given path to give to a Command
pub fn create_named_pipe(path: impl AsRef<Path>) -> std::io::Result<std::process::Stdio> {
	#[cfg(target_family = "unix")]
	{
		use std::os::fd::FromRawFd;

		const O_NONBLOCK: i32 = 0o4000;

		let c_path = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
		unsafe {
			// Read / write mode
			if mkfifo(c_path.as_ptr(), 0o600) != 0 {
				return Err(std::io::Error::last_os_error());
			}
		}

		// Open the file in nonblocking mode

		// O_RDONLY | O_NONBLOCK
		let open_flags = 0 | O_NONBLOCK;

		let file = unsafe { open(c_path.as_ptr(), open_flags, 0) };
		if file == -1 {
			return Err(std::io::Error::last_os_error());
		}

		// Make the file blocking again after opening it
		// F_SETFL
		unsafe { fcntl(file, 4, !O_NONBLOCK) };

		let out = unsafe { std::process::Stdio::from_raw_fd(file) };

		Ok(out)
	}
	#[cfg(target_os = "windows")]
	{
		use std::os::windows::io::FromRawHandle;

		const BUFFER_SIZE: u32 = 512;

		// Open the named pipe if it doesn't already exist
		let c_path = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
		let pipe_handle = unsafe {
			CreateNamedPipeA(
				c_path.as_ptr(),
				0x00000003, // PIPE_ACCESS_DUPLEX
				0x00000000, // PIPE_TYPE_BYTE | PIPE_READMODE_BYTE
				1,          // Max instances
				BUFFER_SIZE,
				BUFFER_SIZE,
				0,
				std::ptr::null_mut(),
			)
		};

		if pipe_handle.is_null() {
			return Err(std::io::Error::last_os_error());
		}

		unsafe {
			if ConnectNamedPipe(pipe_handle, std::ptr::null_mut()) == 0 {
				return Err(std::io::Error::last_os_error());
			}
		}

		unsafe { Ok(std::process::Stdio::from_raw_handle(pipe_handle)) }
	}
}

/// Opens a writeable named pipe at the given path
pub fn open_named_pipe(path: impl AsRef<Path>) -> std::io::Result<File> {
	std::fs::OpenOptions::new()
		.append(true)
		.create(true)
		.open(path)
}
