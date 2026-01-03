use nitro_shared::output::{Message, MessageLevel, NitroOutput};

/// Struct that implements the NitroOutput trait for WASM plugin components
pub struct WASMPluginOutput;

impl WASMPluginOutput {
	/// Create a new WASMPluginOutput
	pub fn new() -> Self {
		Self
	}
}

impl Default for WASMPluginOutput {
	fn default() -> Self {
		Self::new()
	}
}

impl NitroOutput for WASMPluginOutput {
	fn display_text(&mut self, text: String, level: MessageLevel) {
		let level = message_level_to_ordinal(level);
		super::interface::output_display_text(&text, level);
	}

	fn display_message(&mut self, message: Message) {
		let level = message_level_to_ordinal(message.level);
		let Ok(message) = serde_json::to_string(&message.contents) else {
			return;
		};
		super::interface::output_display_message(&message, level);
	}

	fn start_process(&mut self) {
		super::interface::output_start_process();
	}

	fn end_process(&mut self) {
		super::interface::output_end_process();
	}

	fn start_section(&mut self) {
		super::interface::output_start_section();
	}

	fn end_section(&mut self) {
		super::interface::output_end_section();
	}
}

fn message_level_to_ordinal(level: MessageLevel) -> u8 {
	match level {
		MessageLevel::Important => 0,
		MessageLevel::Extra => 1,
		MessageLevel::Debug => 2,
		MessageLevel::Trace => 3,
	}
}
