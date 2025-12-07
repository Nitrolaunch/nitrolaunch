use std::fs::File;

use anyhow::Context;
use nitro_plugin::{
	api::wasm::{net::download_bytes, sys::get_data_dir, WASMPlugin},
	nitro_wasm_plugin,
};
use nitro_shared::{minecraft::VersionManifest, UpdateDepth};

nitro_wasm_plugin!(main, "better_jsons");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.add_versions(|update_depth| {
		let versions_file = get_data_dir()
			.join("internal/better_jsons_manifest.json");

		let versions: VersionManifest =
			if versions_file.exists() && update_depth < UpdateDepth::Full {
				let file = File::open(versions_file)?;
				serde_json::from_reader(file).context("Failed to read cached versions")?
			} else {
				// let mut process = ctx.get_output().get_process();
				// process.display(
				// 	MessageContents::StartProcess("Downloading BetterJSONs manifest".into()),
				// 	MessageLevel::Important,
				// );

				let out = download_bytes("https://raw.githubusercontent.com/MCPHackers/BetterJSONs/refs/heads/main/version_manifest_v2.json".into()).context("Failed to download BetterJSONs manifest")?;

				let versions =  serde_json::from_slice(&out).context("Failed to deserialize better JSONS")?;

				let _ = std::fs::write(versions_file, &out);

				// process.display(
				// 	MessageContents::Success("BetterJSONs manifest downloaded".into()),
				// 	MessageLevel::Important,
				// );

				versions
			};

		Ok(versions.versions)
	})?;

	Ok(())
}
