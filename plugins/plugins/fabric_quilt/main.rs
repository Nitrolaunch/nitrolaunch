use std::path::PathBuf;

use anyhow::{bail, Context};
use nitro_core::io::update::UpdateManager;
use nitro_mods::fabric_quilt;
use nitro_plugin::{api::CustomPlugin, hook::hooks::OnInstanceSetupResult};
use nitro_shared::{loaders::Loader, versions::VersionPattern, UpdateDepth};

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("fabric_quilt", include_str!("plugin.json"))?;
	plugin.on_instance_setup(|mut ctx, arg| {
		let Some(side) = arg.side else {
			bail!("Instance side is empty");
		};

		if arg.config.disable_loader_update {
			return Ok(OnInstanceSetupResult::default());
		}

		// Make sure this is a Fabric or Quilt instance
		if arg.loader != Loader::Fabric && arg.loader != Loader::Quilt {
			return Ok(OnInstanceSetupResult::default());
		}

		let mode = if arg.loader == Loader::Fabric {
			fabric_quilt::Mode::Fabric
		} else {
			fabric_quilt::Mode::Quilt
		};

		let internal_dir = PathBuf::from(arg.internal_dir);

		let manager = UpdateManager::new(UpdateDepth::Full);

		let client = nitro_net::download::Client::new();

		let runtime = tokio::runtime::Runtime::new()?;

		let desired_fq_version = arg.desired_loader_version.and_then(|x| {
			if let VersionPattern::Single(pat) = x {
				Some(pat)
			} else {
				None
			}
		});

		let meta = runtime
			.block_on(fabric_quilt::get_meta(
				&arg.version_info.version,
				desired_fq_version.as_deref(),
				&mode,
				&internal_dir,
				&manager,
				&client,
			))
			.context("Failed to get metadata")?;

		let libraries_dir = internal_dir.join("libraries");

		runtime
			.block_on(fabric_quilt::download_files(
				&meta,
				&libraries_dir,
				mode,
				&manager,
				&client,
				ctx.get_output(),
			))
			.context("Failed to download common files")?;

		runtime
			.block_on(fabric_quilt::download_side_specific_files(
				&meta,
				&libraries_dir,
				side,
				&manager,
				&client,
			))
			.context("Failed to download side-specific files")?;

		let classpath = fabric_quilt::get_classpath(&meta, &libraries_dir, side)
			.context("Failed to get classpath")?;

		let main_class = meta
			.launcher_meta
			.main_class
			.get_main_class_string(side)
			.to_string();

		Ok(OnInstanceSetupResult {
			main_class_override: Some(main_class),
			classpath_extension: classpath.get_entries().to_vec(),
			jvm_args: vec!["-Dsodium.checks.issue2561=false".to_string()],
			..Default::default()
		})
	})?;

	plugin.get_loader_versions(|ctx, arg| {
		if arg.loader != Loader::Fabric && arg.loader != Loader::Quilt {
			return Ok(Vec::new());
		}

		let runtime = tokio::runtime::Runtime::new()?;
		let internal_dir = PathBuf::from(ctx.get_data_dir()?.join("internal"));
		let manager = UpdateManager::new(UpdateDepth::Full);
		let client = nitro_net::download::Client::new();

		let mode = if arg.loader == Loader::Fabric {
			fabric_quilt::Mode::Fabric
		} else {
			fabric_quilt::Mode::Quilt
		};

		let meta = runtime
			.block_on(fabric_quilt::get_all_meta(
				&arg.minecraft_version,
				&mode,
				&internal_dir,
				&manager,
				&client,
			))
			.context("Failed to get metadata")?;

		let out = meta.into_iter().map(|version| {
			version
				.loader
				.maven
				.replace("net.fabricmc:fabric-loader:", "")
		});

		Ok(out.collect())
	})?;

	Ok(())
}
