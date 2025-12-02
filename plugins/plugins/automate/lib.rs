use anyhow::{bail, Context};
use nitro_plugin::{
	api::wasm::{sys::get_os_string, WASMPlugin},
	nitro_wasm_plugin,
};

nitro_wasm_plugin!(main, "automate");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.on_instance_launch(|arg| {
		if let Some(cmd) = arg.config.plugin_config.get("on_launch") {
			let cmd: String =
				serde_json::from_value(cmd.clone()).context("Invalid command format")?;

			run_hook(&cmd).context("Failed to run script")?;
		}

		Ok(())
	})?;

	plugin.on_instance_stop(|arg| {
		if let Some(cmd) = arg.config.plugin_config.get("on_stop") {
			let cmd: String =
				serde_json::from_value(cmd.clone()).context("Invalid command format")?;

			run_hook(&cmd).context("Failed to run script")?;
		}

		Ok(())
	})?;

	Ok(())
}

fn run_hook(cmd: &str) -> anyhow::Result<()> {
	match get_os_string().as_str() {
		"linux" | "macos" => {
			let shell = std::env::var("SHELL").unwrap_or("/bin/sh".into());

			let mut command = std::process::Command::new(shell);
			command.arg("-c");
			command.arg(cmd);

			let success = command.spawn()?.wait()?.success();
			if !success {
				bail!("Command returned a non-zero exit code");
			}
		}
		_ => {
			println!("Cannot run Automate plugin on this platform");
		}
	}

	Ok(())
}
