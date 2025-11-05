use anyhow::{bail, Context};
use nitro_core::Paths;
use nitro_mods::sponge;
use nitro_plugin::{api::CustomPlugin, hooks::OnInstanceSetupResult};
use nitro_shared::{loaders::Loader, Side};

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("sponge", include_str!("plugin.json"))?;
	plugin.on_instance_setup(|_, arg| {
		let Some(side) = arg.side else {
			bail!("Instance side is empty");
		};

		// Make sure this is a Sponge server instance
		if side != Side::Server || arg.loader != Loader::Sponge {
			return Ok(OnInstanceSetupResult::default());
		}

		let paths = Paths::new().context("Failed to create paths")?;

		let client = nitro_net::download::Client::new();

		let runtime = tokio::runtime::Runtime::new()?;

		let mode = sponge::Mode::Vanilla;

		let artifacts = runtime
			.block_on(sponge::get_artifacts(
				mode,
				&arg.version_info.version,
				&client,
			))
			.context("Failed to get list of Sponge versions")?;

		let artifact = if let Some(version) = arg.desired_loader_version {
			version
				.get_match(&artifacts)
				.context("Sponge version does not exist")?
		} else {
			artifacts
				.last()
				.context("Failed to find a valid Sponge version")?
				.clone()
		};

		let artifact_info = runtime
			.block_on(sponge::get_artifact_info(mode, &artifact, &client))
			.context("Failed to get artifact info")?;

		runtime
			.block_on(sponge::download_server_jar(
				sponge::Mode::Vanilla,
				&arg.version_info.version,
				&artifact_info,
				&paths,
				&client,
			))
			.context("Failed to download Sponge server JAR")?;

		let jar_path =
			sponge::get_local_jar_path(sponge::Mode::Vanilla, &arg.version_info.version, &paths);
		let main_class = sponge::SPONGE_SERVER_MAIN_CLASS;

		Ok(OnInstanceSetupResult {
			main_class_override: Some(main_class.into()),
			jar_path_override: Some(jar_path.to_string_lossy().to_string()),
			..Default::default()
		})
	})?;

	plugin.get_loader_versions(|_, arg| {
		if arg.loader != Loader::Sponge {
			return Ok(Vec::new());
		}

		let client = nitro_net::download::Client::new();
		let runtime = tokio::runtime::Runtime::new()?;

		let mode = sponge::Mode::Vanilla;

		let artifacts = runtime
			.block_on(sponge::get_artifacts(mode, &arg.minecraft_version, &client))
			.context("Failed to get list of Sponge versions")?;

		Ok(artifacts)
	})?;

	Ok(())
}
