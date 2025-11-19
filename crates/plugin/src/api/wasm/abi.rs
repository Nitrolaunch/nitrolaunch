use crate::api::wasm::util::PtrAndLength;

#[link(wasm_import_module = "nitro")]
extern "C" {
	/// 0 if null
	pub fn get_custom_config() -> PtrAndLength;
}
