use anyhow::Context;
use nitro_core::{
	io::{json_from_file, json_to_file},
	net::game_files::version_manifest::VersionManifest,
};
use nitro_net::download::{self, Client};
use nitro_plugin::api::CustomPlugin;
use nitro_shared::{
	output::{MessageContents, MessageLevel, NitroOutput},
	UpdateDepth,
};

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("better_jsons", include_str!("plugin.json"))?;
	plugin.add_versions(|mut ctx, update_depth| {
		let versions_file = ctx
			.get_data_dir()?
			.join("internal/better_jsons_manifest.json");

		let versions: VersionManifest =
			if versions_file.exists() && update_depth < UpdateDepth::Full {
				json_from_file(versions_file).context("Failed to read cached versions")?
			} else {
				let runtime = tokio::runtime::Runtime::new()?;
				let client = Client::new();

				let mut process = ctx.get_output().get_process();
				process.display(
					MessageContents::StartProcess("Downloading BetterJSONs manifest".into()),
					MessageLevel::Important,
				);

				let out = runtime.block_on(download::json(
					"https://raw.githubusercontent.com/MCPHackers/BetterJSONs/refs/heads/main/version_manifest_v2.json",
					&client,
				))?;

				let _ = json_to_file(versions_file, &out);

				process.display(
					MessageContents::Success("BetterJSONs manifest downloaded".into()),
					MessageLevel::Important,
				);

				out
			};

		Ok(versions.versions)
	})?;

	Ok(())
}
