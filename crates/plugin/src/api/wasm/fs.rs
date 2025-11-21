use std::path::{Path, PathBuf};

use crate::{
	api::wasm::util::{
		create_abi_buffer, create_abi_string, read_result_wasm_string, read_wasm_string_2,
	},
	PtrAndLength,
};

/// Gets the Nitrolaunch data dir
pub fn get_data_dir() -> PathBuf {
	let path = unsafe { read_wasm_string_2(super::abi::get_data_dir()) };
	PathBuf::from(path)
}

/// Gets the Nitrolaunch config dir
pub fn get_config_dir() -> PathBuf {
	let path = unsafe { read_wasm_string_2(super::abi::get_config_dir()) };
	PathBuf::from(path)
}

/// Checks whether a path exists
pub fn path_exists(path: impl AsRef<Path>) -> bool {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::path_exists(path.ptr, path.len) };

	result != 0
}

/// Creates a file at the given path
pub fn create_file(path: impl AsRef<Path>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::create_file(path.ptr, path.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}

/// Removes a file at the given path
pub fn remove(path: impl AsRef<Path>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::remove_file(path.ptr, path.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}

/// Writes to a file at the given path
pub fn write(path: impl AsRef<Path>, contents: Vec<u8>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let data = create_abi_buffer(contents);

	let result = unsafe { super::abi::write_file(path.ptr, path.len, data.ptr, data.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}

/// Reads a file at the given path
pub fn read(path: impl AsRef<Path>) -> Result<&'static [u8], &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let (data_ptr, data_len, err_ptr, err_len) =
		unsafe { super::abi::read_file(path.ptr, path.len) };

	let result = PtrAndLength {
		ptr: err_ptr,
		len: err_len,
	};
	let result = unsafe { read_result_wasm_string(result) };

	let data = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, data_len as usize) };

	result.map(|_| data)
}

/// Creates a directory at the given path
pub fn create_dir(path: impl AsRef<Path>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::create_dir(path.ptr, path.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}

/// Creates a directory and all parents at the given path
pub fn create_dir_all(path: impl AsRef<Path>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::create_dir_all(path.ptr, path.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}

/// Creates all directories leading to the given path
pub fn create_leading_dirs(path: impl AsRef<Path>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::create_leading_dirs(path.ptr, path.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}

/// Removes a directory at the given path
pub fn remove_dir(path: impl AsRef<Path>) -> Result<(), &'static str> {
	let path = path.as_ref().to_string_lossy().to_string();
	let path = create_abi_string(path);

	let result = unsafe { super::abi::remove_dir(path.ptr, path.len) };
	let result = unsafe { read_result_wasm_string(result) };

	result
}
