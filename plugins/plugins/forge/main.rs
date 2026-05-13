use std::path::PathBuf;

use anyhow::{Context, bail};
use nitro_core::io::{json_from_file, json_to_file};
use nitro_mods::forge::{self, Mode};
use nitro_net::neoforge;
use nitro_plugin::{api::executable::ExecutablePlugin, hook::hooks::OnInstanceSetupResult};
use nitro_shared::{
	UpdateDepth,
	loaders::Loader,
	output::{MessageContents, NitroOutput},
};

fn main() -> anyhow::Result<()> {
	let mut plugin = ExecutablePlugin::from_manifest_file("forge", include_str!("plugin.json"))?;
	plugin.on_instance_setup(|mut ctx, arg| {
		let Some(side) = arg.side else {
			bail!("Instance side is empty");
		};

		if arg.config.custom_launch {
			return Ok(OnInstanceSetupResult::default());
		}

		if arg.loader != Loader::NeoForged {
			return Ok(OnInstanceSetupResult::default());
		}

		let mode = Mode::NeoForge;
		let internal_dir = PathBuf::from(arg.internal_dir);
		let client = nitro_net::download::Client::new();
		let runtime = tokio::runtime::Runtime::new()?;
		let loader_version;

		let result = match mode {
			Mode::NeoForge => {
				let versions_path = internal_dir.join("forge/neoforge_versions.json");
				let versions = if arg.update_depth < UpdateDepth::Full && versions_path.exists() {
					json_from_file(&versions_path)?
				} else {
					let mut process = ctx.get_output().get_process();
					process.display(MessageContents::StartProcess(format!(
						"Fetching {mode} versions"
					)));

					let versions = runtime.block_on(neoforge::get_versions(&client))?;
					json_to_file(versions_path, &versions)?;

					process.display(MessageContents::Success(format!("{mode} versions fetched")));

					versions
				};

				let version =
					neoforge::get_latest_neoforge_version(&versions, &arg.version_info.version)
						.context("Could not find NeoForge version for this Minecraft version")?;

				loader_version = Some(version.clone());

				runtime
					.block_on(forge::install(
						&client,
						&internal_dir,
						arg.update_depth,
						&arg.version_info,
						side,
						mode,
						version,
						&PathBuf::from(arg.jvm_path),
						ctx.get_output(),
					))
					.context("Failed to install NeoForge")?
			}
		};

		Ok(OnInstanceSetupResult {
			classpath_extension: result.classpath.get_entries().to_vec(),
			main_class_override: Some(result.main_class),
			jvm_args: result.jvm_args,
			game_args: result.game_args,
			loader_version,
			exclude_game_jar: true,
			..Default::default()
		})
	})?;

	Ok(())
}
