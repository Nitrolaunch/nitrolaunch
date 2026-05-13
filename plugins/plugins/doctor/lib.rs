use std::path::PathBuf;

use nitro_plugin::api::wasm::output::WASMPluginOutput;
use nitro_plugin::api::wasm::{WASMPlugin, sys::get_os_string};
use nitro_plugin::nitro_wasm_plugin;
use nitro_shared::Side;
use nitro_shared::output::{MessageContents, NitroOutput};

use crate::signature::Diagnosis;

mod signature;

nitro_wasm_plugin!(main, "doctor");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.on_instance_stop(|arg| {
		let Some(dir) = arg.inst_dir else {
			return Ok(());
		};

		let dir = PathBuf::from(dir);
		let log_path = dir.join("logs/latest.log");
		if !log_path.exists() {
			return Ok(());
		}

		let Ok(log_file) = std::fs::read_to_string(log_path) else {
			return Ok(());
		};

		let diagnoses = diagnose(&log_file, arg.side.unwrap_or_default());

		if !diagnoses.is_empty() {
			let mut o = WASMPluginOutput::new();
			o.display(MessageContents::Header("Potential Problems:".into()));
			for diagnosis in diagnoses {
				diagnosis.output(&mut o);
			}
		}

		Ok(())
	})?;

	Ok(())
}

fn diagnose(log_file: &str, side: Side) -> Vec<Diagnosis> {
	let diagnoses: Vec<Diagnosis> =
		serde_json::from_slice(include_bytes!("signatures.json")).unwrap();

	let os = get_os_string();

	let mut out = Vec::new();
	for diagnosis in diagnoses {
		if diagnosis.signature.matches(log_file, side, &os) {
			out.push(diagnosis);
		}
	}

	out
}
