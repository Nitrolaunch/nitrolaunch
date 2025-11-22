use crate::api::wasm::util::read_wasm_string_2;

/// Gets the operating system as a lowercase string
pub fn get_os_string() -> &'static str {
	unsafe { read_wasm_string_2(super::abi::get_os_string()) }
}

/// Gets the system architecture as a lowercase string
pub fn get_arch_string() -> &'static str {
	unsafe { read_wasm_string_2(super::abi::get_arch_string()) }
}

/// Gets the system pointer width
pub fn get_pointer_width() -> u32 {
	unsafe { super::abi::get_pointer_width() }
}
