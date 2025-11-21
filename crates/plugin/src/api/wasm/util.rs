use std::alloc::Layout;

use crate::{api::wasm::HOOK_RESULT, PtrAndLength};

/// Allocates a buffer in WASM
#[no_mangle]
pub extern "C" fn nitro_plugin_alloc(size: usize) -> *mut u8 {
	let layout = Layout::from_size_align(size, 1).expect("Invalid layout");
	unsafe { std::alloc::alloc(layout) }
}

/// Deallocates a buffer that was allocated in WASM
#[no_mangle]
pub extern "C" fn nitro_plugin_dealloc(ptr: *mut u8, size: usize) {
	let layout = Layout::from_size_align(size, 1).expect("Invalid layout");
	unsafe {
		std::alloc::dealloc(ptr, layout);
	}
}

/// Gets the pointer to the hook result
#[no_mangle]
pub extern "C" fn nitro_plugin_get_hook_result() -> *mut u8 {
	#[allow(static_mut_refs)]
	unsafe {
		HOOK_RESULT.as_mut_ptr()
	}
}

/// Gets the length of the hook result
#[no_mangle]
pub extern "C" fn nitro_plugin_get_hook_result_len() -> usize {
	#[allow(static_mut_refs)]
	unsafe {
		HOOK_RESULT.len()
	}
}

/// Reads a string pointer passed through WASM. Remember to free it later!
pub unsafe fn read_wasm_string(ptr: *const u8, size: usize) -> &'static str {
	let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
	std::str::from_utf8(slice).expect("Invalid UTF-8")
}

/// Reads a string pointer passed through WASM from a PtrAndLength struct. Remember to free it later!
pub unsafe fn read_wasm_string_2(ptr_and_length: PtrAndLength) -> &'static str {
	read_wasm_string(ptr_and_length.ptr as *const u8, ptr_and_length.len as usize)
}

/// Reads an optional string pointer passed through WASM from a PtrAndLength struct. Remember to free it later!
pub unsafe fn read_optional_wasm_string(ptr_and_length: PtrAndLength) -> Option<&'static str> {
	if ptr_and_length.ptr == 0 {
		None
	} else {
		Some(read_wasm_string_2(ptr_and_length))
	}
}

/// Reads an result string pointer passed through WASM from a PtrAndLength struct. Remember to free it later!
pub unsafe fn read_result_wasm_string(ptr_and_length: PtrAndLength) -> Result<(), &'static str> {
	if ptr_and_length.ptr == 0 {
		Ok(())
	} else {
		Err(read_wasm_string_2(ptr_and_length))
	}
}

/// Creates an abi-compatible string by making it static and returning the pointer and length
pub fn create_abi_string(string: String) -> PtrAndLength {
	let (ptr, len) = (string.as_ptr() as u32, string.len() as u32);
	std::mem::forget(string);

	PtrAndLength { ptr, len }
}

/// Creates an abi-compatible buffer by making it static and returning the pointer and length
pub fn create_abi_buffer(buf: Vec<u8>) -> PtrAndLength {
	let (ptr, len) = (buf.as_ptr() as u32, buf.len() as u32);
	std::mem::forget(buf);

	PtrAndLength { ptr, len }
}

/// Gets the custom config for this plugin
pub fn get_custom_config() -> Option<&'static str> {
	// Some("{\"instances\":{}}")
	let ptr_and_length = unsafe { super::abi::get_custom_config() };
	if ptr_and_length.ptr == 0 {
		None
	} else {
		Some(unsafe { read_wasm_string_2(ptr_and_length) })
	}
}
