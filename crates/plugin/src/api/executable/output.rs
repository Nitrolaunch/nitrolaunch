use std::io::{Stdout, Write};

use nitro_shared::output::{Message, MessageLevel, NitroOutput};

use crate::{input_output::OutputAction, plugin::NEWEST_PROTOCOL_VERSION};

/// Struct that implements the NitroOutput trait for printing serialized messages
/// to stdout for the plugin runner to read
pub struct ExecutablePluginOutput {
	use_base64: bool,
	protocol_version: u16,
	stdout: Stdout,
}

impl ExecutablePluginOutput {
	/// Create a new ExecutablePluginOutput
	pub fn new(use_base64: bool, protocol_version: u16) -> Self {
		Self {
			use_base64,
			protocol_version,
			stdout: std::io::stdout(),
		}
	}
}

impl Default for ExecutablePluginOutput {
	fn default() -> Self {
		Self::new(true, NEWEST_PROTOCOL_VERSION)
	}
}

impl NitroOutput for ExecutablePluginOutput {
	fn display_text(&mut self, text: String, level: MessageLevel) {
		let action = OutputAction::Text(text, level);
		if let Ok(text) = action.serialize(self.use_base64, self.protocol_version) {
			let _ = writeln!(&mut self.stdout, "{text}");
		}
	}

	fn display_message(&mut self, message: Message) {
		let action = OutputAction::Message(message);
		if let Ok(text) = action.serialize(self.use_base64, self.protocol_version) {
			let _ = writeln!(&mut self.stdout, "{text}");
		}
	}

	fn start_process(&mut self) {
		let action = OutputAction::StartProcess;
		if let Ok(text) = action.serialize(self.use_base64, self.protocol_version) {
			let _ = writeln!(&mut self.stdout, "{text}");
		}
	}

	fn end_process(&mut self) {
		let action = OutputAction::EndProcess;
		if let Ok(text) = action.serialize(self.use_base64, self.protocol_version) {
			let _ = writeln!(&mut self.stdout, "{text}");
		}
	}

	fn start_section(&mut self) {
		let action = OutputAction::StartSection;
		if let Ok(text) = action.serialize(self.use_base64, self.protocol_version) {
			let _ = writeln!(&mut self.stdout, "{text}");
		}
	}

	fn end_section(&mut self) {
		let action = OutputAction::EndSection;
		if let Ok(text) = action.serialize(self.use_base64, self.protocol_version) {
			let _ = writeln!(&mut self.stdout, "{text}");
		}
	}
}
