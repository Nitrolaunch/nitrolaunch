use anyhow::{bail, Context};
use nitro_plugin::{
	api::wasm::{
		net::download_file,
		sys::{get_data_dir, get_os_string},
		WASMPlugin,
	},
	hook::hooks::OnInstanceSetupResult,
	nitro_wasm_plugin,
};
use nitro_shared::UpdateDepth;
use serde_json::Value;

nitro_wasm_plugin!(main, "glfw_fix");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.on_instance_setup(|arg| {
		if arg.game_dir.is_none() {
			return Ok(OnInstanceSetupResult::default());
		};

		let enabled = arg
			.config
			.plugin_config
			.get("fix_glfw")
			.is_some_and(|x| x == &Value::Bool(true));

		if !enabled {
			return Ok(OnInstanceSetupResult::default());
		}

		let (filename, url) = match get_os_string().as_str() {
			"linux" | "macos" => (
				"libglfw.so",
				"https://github.com/Frontear/glfw-libs/releases/download/2024-08-31/libglfw.so",
			),
			"windows" => (
				"glfw3.dll",
				"https://github.com/Frontear/glfw-libs/releases/download/2024-08-31/glfw3.dll",
			),
			_ => bail!("Unsupported operating system"),
		};

		let lib_path = get_data_dir().join(format!("internal/{filename}"));

		let output = OnInstanceSetupResult {
			jvm_args: vec![format!(
				"-Dorg.lwjgl.glfw.libname={}",
				lib_path.to_string_lossy()
			)],
			..Default::default()
		};

		if lib_path.exists() || arg.update_depth == UpdateDepth::Force {
			return Ok(output);
		}

		// let mut process = ctx.get_output().get_process();
		// process.display(
		// 	MessageContents::StartProcess("Downloading GLFW".into()),
		// 	MessageLevel::Important,
		// );

		download_file(url.to_string(), lib_path).context("Failed to download GLFW")?;

		// process.display(
		// 	MessageContents::Success("GLFW downloaded".into()),
		// 	MessageLevel::Important,
		// );

		Ok(output)
	})?;

	Ok(())
}
