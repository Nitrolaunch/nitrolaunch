use crate::PtrAndLength;

#[link(wasm_import_module = "nitro")]
extern "C" {
	/// 0 if null
	pub fn get_custom_config() -> PtrAndLength;
	pub fn get_data_dir() -> PtrAndLength;
	pub fn get_config_dir() -> PtrAndLength;
	pub fn path_exists(path: u32, path_len: u32) -> u32;
	/// Returns error pointer
	pub fn create_file(path: u32, path_len: u32) -> PtrAndLength;
	/// Returns error pointer
	pub fn remove_file(path: u32, path_len: u32) -> PtrAndLength;
	/// Returns error pointer
	pub fn write_file(path: u32, path_len: u32, data: u32, data_len: u32) -> PtrAndLength;
	/// Returns result and error pointer
	#[allow(improper_ctypes)]
	pub fn read_file(path: u32, path_len: u32) -> (u32, u32, u32, u32);
	/// Returns error pointer
	pub fn create_dir(path: u32, path_len: u32) -> PtrAndLength;
	/// Returns error pointer
	pub fn create_dir_all(path: u32, path_len: u32) -> PtrAndLength;
	/// Returns error pointer
	pub fn create_leading_dirs(path: u32, path_len: u32) -> PtrAndLength;
	/// Returns error pointer
	pub fn remove_dir(path: u32, path_len: u32) -> PtrAndLength;
}
