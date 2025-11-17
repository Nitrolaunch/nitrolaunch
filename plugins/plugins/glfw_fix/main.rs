use anyhow::Context;
use nitro_net::download::{self, Client};
use nitro_plugin::{api::CustomPlugin, hook::hooks::OnInstanceSetupResult};
use nitro_shared::{
	output::{MessageContents, MessageLevel, NitroOutput},
	UpdateDepth,
};
use serde_json::Value;

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("glfw_fix", include_str!("plugin.json"))?;
	plugin.on_instance_setup(|mut ctx, arg| {
		let enabled = arg
			.config
			.common
			.plugin_config
			.get("fix_glfw")
			.is_some_and(|x| x == &Value::Bool(true));

		if !enabled {
			return Ok(OnInstanceSetupResult::default());
		}

		#[cfg(target_family = "unix")]
		let (filename, url) = (
			"libglfw.so",
			"https://github.com/Frontear/glfw-libs/releases/download/2024-08-31/libglfw.so",
		);
		#[cfg(target_os = "windows")]
		let (filename, url) = (
			"glfw3.dll",
			"https://github.com/Frontear/glfw-libs/releases/download/2024-08-31/glfw3.dll",
		);

		let lib_path = ctx.get_data_dir()?.join(format!("internal/{filename}"));

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

		let mut process = ctx.get_output().get_process();
		process.display(
			MessageContents::StartProcess("Downloading GLFW".into()),
			MessageLevel::Important,
		);

		let runtime = tokio::runtime::Runtime::new()?;
		let client = Client::new();

		runtime
			.block_on(download::file(url, lib_path, &client))
			.context("Failed to download GLFW")?;

		process.display(
			MessageContents::Success("GLFW downloaded".into()),
			MessageLevel::Important,
		);

		Ok(output)
	})?;

	Ok(())
}
