use std::{
	fs::File,
	path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use nitro_config::instance::InstanceConfig;
use nitro_instance::addon::{
	modpack::{
		mrpack::{ModrinthIndex, ModrinthPack},
		LinkMethod, Modpack,
	},
	storage::get_sha256_addon_path,
};
use nitro_plugin::{
	api::wasm::{
		net::download_files,
		output::WASMPluginOutput,
		sys::{get_data_dir, update_link},
		WASMPlugin,
	},
	hook::hooks::ImportInstanceResult,
	nitro_wasm_plugin,
};
use nitro_shared::{
	output::{MessageContents, NitroOutput},
	pkg::PackageOverrides,
	versions::MinecraftVersionDeser,
	Side,
};

nitro_wasm_plugin!(main, "modpacks");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.import_instance(|arg| {
		let source_path = PathBuf::from(arg.source_path);
		let target_path = PathBuf::from(arg.result_path);

		let addons_dir = get_data_dir().join("internal/addons");

		let mut output = WASMPluginOutput::new();

		match arg.format.as_str() {
			"mrpack" => {
				let side = arg.side.context("Side not specified")?;

				let file = File::open(source_path).context("Failed to open pack file")?;
				let mut modpack =
					ModrinthPack::from_stream(file).context("Failed to open mrpack")?;
				modpack.set_link_method(Box::new(WasmLinkMethod));

				// Download files
				let mut process = output.get_process();
				process.display(MessageContents::StartProcess("Downloading mods".into()));

				let mut urls = Vec::new();
				let mut paths = Vec::new();
				for file in &modpack.index().files {
					let Some(url) = file.downloads.first() else {
						continue;
					};

					let path = get_sha256_addon_path(&addons_dir, &file.hashes.sha512);
					if let Some(parent) = path.parent() {
						let _ = std::fs::create_dir_all(parent);
					}

					urls.push(url.clone());
					paths.push(path.to_string_lossy().to_string());
				}

				download_files(&urls, &paths, true).context("Failed to download modpack files")?;

				process.display(MessageContents::Success("Mods downloaded".into()));
				std::mem::drop(process);

				let target_path = match side {
					Side::Client => target_path.join(".minecraft"),
					Side::Server => target_path,
				};

				let mut process = output.get_process();
				process.display(MessageContents::StartProcess("Installing modpack".into()));
				modpack
					.apply(&target_path, &addons_dir, side, true)
					.context("Failed to install modpack")?;
				process.display(MessageContents::Success("Modpack installed".into()));
				std::mem::drop(process);

				let config = mrpack_index_to_config(modpack.index(), side);

				Ok(ImportInstanceResult {
					format: arg.format,
					config,
				})
			}
			other => bail!("Unsupported format {other}"),
		}
	})?;

	Ok(())
}

/// Creates InstanceConfig from an mrpack index
fn mrpack_index_to_config(index: &ModrinthIndex, side: Side) -> InstanceConfig {
	// Suppress mods that this pack provides
	let mut suppress = Vec::new();
	for file in &index.files {
		let (Some(project_id), _) = file.get_modrinth_info() else {
			continue;
		};

		suppress.push(format!("modrinth:{project_id}"));
	}

	let loader = if let Some(version) = &index.dependencies.forge {
		Some(format!("forge@{version}"))
	} else if let Some(version) = &index.dependencies.neoforge {
		Some(format!("neoforged@{version}"))
	} else if let Some(version) = &index.dependencies.fabric_loader {
		Some(format!("fabric@{version}"))
	} else if let Some(version) = &index.dependencies.quilt_loader {
		Some(format!("quilt@{version}"))
	} else {
		None
	};

	InstanceConfig {
		side: Some(side),
		name: Some(index.name.clone()),
		version: Some(MinecraftVersionDeser::Version(
			index.dependencies.minecraft.clone().into(),
		)),
		loader,
		overrides: PackageOverrides {
			suppress,
			..Default::default()
		},
		..Default::default()
	}
}

struct WasmLinkMethod;

impl LinkMethod for WasmLinkMethod {
	fn link(&self, original: &Path, link: &Path) -> anyhow::Result<()> {
		update_link(link, original)?;
		Ok(())
	}
}
