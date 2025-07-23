use std::{
	ops::DerefMut,
	path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use nitro_core::{
	io::{files::create_leading_dirs, json_from_file, json_to_file},
	Paths,
};
use nitro_mods::paper::{self, BuildInfoResponse};
use nitro_net::download::Client;
use nitro_plugin::{api::CustomPlugin, hooks::OnInstanceSetupResult};
use nitro_shared::{
	loaders::Loader,
	output::{NitroOutput, MessageContents, MessageLevel, OutputProcess},
	versions::VersionPattern,
	Side, UpdateDepth,
};
use tokio::runtime::Runtime;

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("paper", include_str!("plugin.json"))?;
	plugin.on_instance_setup(|mut ctx, arg| {
		let Some(side) = arg.side else {
			bail!("Instance side is empty");
		};

		// Make sure this is a Paper or Folia server instance
		if side != Side::Server || (arg.loader != Loader::Paper && arg.loader != Loader::Folia) {
			return Ok(OnInstanceSetupResult::default());
		}

		let mode = if arg.loader == Loader::Paper {
			paper::Mode::Paper
		} else {
			paper::Mode::Folia
		};

		let mut process = OutputProcess::new(ctx.get_output());
		process.display(
			MessageContents::StartProcess(format!("Checking for {mode} updates")),
			MessageLevel::Important,
		);

		let client = nitro_net::download::Client::new();
		let paths = Paths::new()?;

		let runtime = tokio::runtime::Runtime::new()?;

		// Check if this Minecraft version is available
		let stored_versions_path = get_stored_versions_path(&paths, mode);
		let versions = if stored_versions_path.exists() && arg.update_depth == UpdateDepth::Shallow
		{
			process.display(
				MessageContents::StartProcess(format!("Downloading version list")),
				MessageLevel::Important,
			);
			json_from_file(&stored_versions_path).context("Failed to read versions from file")?
		} else {
			runtime
				.block_on(paper::get_all_versions(mode, &client))
				.context("Failed to get list of versions")?
		};
		let _ = create_leading_dirs(&stored_versions_path);
		json_to_file(stored_versions_path, &versions)
			.context("Failed to write versions to file")?;

		if !versions.iter().any(|x| *x == arg.version_info.version) {
			bail!("Could not find a Paper version for the given Minecraft version");
		}

		// Get the build numbers (actual project versions)
		let builds_path = get_stored_builds_path(&paths, mode, &arg.version_info.version);
		let build_nums = if builds_path.exists() && arg.update_depth == UpdateDepth::Shallow {
			json_from_file(&builds_path).context("Failed to read builds from file")?
		} else {
			process.display(
				MessageContents::StartProcess(format!("Getting build list")),
				MessageLevel::Important,
			);
			runtime
				.block_on(paper::get_builds(mode, &arg.version_info.version, &client))
				.with_context(|| {
					format!("Failed to get list of build numbers for {mode} project")
				})?
		};
		let _ = create_leading_dirs(&builds_path);
		json_to_file(builds_path, &build_nums).context("Failed to write builds to file")?;

		let build_nums_strings: Vec<_> = build_nums.iter().map(|x| x.to_string()).collect();

		let desired_version = arg
			.desired_loader_version
			.unwrap_or(VersionPattern::Any)
			.get_match(&build_nums_strings)
			.with_context(|| format!("Failed to find the given {mode} version"))?;
		let desired_build_num: u16 = desired_version
			.parse()
			.context("The desired version must be a an unsigned integer")?;

		let current_build_num: Option<u16> =
			arg.current_loader_version.and_then(|x| x.parse().ok());

		// If the new and current build nums mismatch, then get info for the current build num and
		// use it to teardown
		if let Some(current_build_num) = current_build_num {
			if desired_build_num != current_build_num {
				process.display(
					MessageContents::StartProcess(format!("Removing old build")),
					MessageLevel::Important,
				);
				let build_info = get_build_info(
					&paths,
					mode,
					&arg.version_info.version,
					current_build_num,
					arg.update_depth,
					&runtime,
					&client,
					process.deref_mut(),
				)
				.context("Failed to get old version build info")?;

				remove_paper(
					&PathBuf::from(arg.game_dir),
					build_info.downloads.application.name,
				)
				.with_context(|| format!("Failed to remove {mode} from the instance"))?;
			}
		}

		// Get the name of the remote JAR file we need to download
		let build_info = get_build_info(
			&paths,
			mode,
			&arg.version_info.version,
			desired_build_num,
			arg.update_depth,
			&runtime,
			&client,
			process.deref_mut(),
		)
		.context("Failed to get build info")?;

		// Download the JAR
		let jar_path = paper::get_local_jar_path(mode, &arg.version_info.version, &paths);
		if !jar_path.exists() || arg.update_depth == UpdateDepth::Force {
			process.display(
				MessageContents::StartProcess(format!("Downloading JAR file")),
				MessageLevel::Important,
			);
			runtime
				.block_on(paper::download_server_jar(
					mode,
					&arg.version_info.version,
					desired_build_num,
					&build_info.downloads.application.name,
					&paths,
					&client,
				))
				.with_context(|| format!("Failed to download JAR file for {mode}"))?;
		}

		process.display(
			MessageContents::Success(format!("{mode} updated")),
			MessageLevel::Important,
		);

		let main_class = paper::PAPER_SERVER_MAIN_CLASS;

		Ok(OnInstanceSetupResult {
			main_class_override: Some(main_class.into()),
			jar_path_override: Some(jar_path.to_string_lossy().to_string()),
			loader_version: Some(desired_build_num.to_string()),
			..Default::default()
		})
	})?;

	Ok(())
}

fn get_stored_versions_path(paths: &Paths, mode: paper::Mode) -> PathBuf {
	paths
		.internal
		.join(format!("paper/{}/versions.json", mode.to_str()))
}

fn get_stored_builds_path(paths: &Paths, mode: paper::Mode, version: &str) -> PathBuf {
	paths
		.internal
		.join(format!("paper/{}/{version}_builds.json", mode.to_str()))
}

fn get_stored_build_info_path(
	paths: &Paths,
	mode: paper::Mode,
	version: &str,
	build: u16,
) -> PathBuf {
	paths
		.internal
		.join(format!("paper/{}/{version}_{build}.json", mode.to_str()))
}

fn get_build_info(
	paths: &Paths,
	mode: paper::Mode,
	version: &str,
	build: u16,
	update_depth: UpdateDepth,
	runtime: &Runtime,
	client: &Client,
	o: &mut impl NitroOutput,
) -> anyhow::Result<BuildInfoResponse> {
	let build_info_path = get_stored_build_info_path(&paths, mode, version, build);
	let build_info = if build_info_path.exists() && update_depth <= UpdateDepth::Full {
		json_from_file(&build_info_path).context("Failed to read build info from file")?
	} else {
		o.display(
			MessageContents::StartProcess(format!("Downloading build info")),
			MessageLevel::Important,
		);
		runtime
			.block_on(paper::get_build_info(mode, version, build, client))
			.with_context(|| format!("Failed to get build info for new {mode} version"))?
	};
	let _ = create_leading_dirs(&build_info_path);
	json_to_file(build_info_path, &build_info).context("Failed to write build info to file")?;

	Ok(build_info)
}

fn remove_paper(game_dir: &Path, paper_file_name: String) -> anyhow::Result<()> {
	let paper_path = game_dir.join(paper_file_name);
	if paper_path.exists() {
		std::fs::remove_file(paper_path).context("Failed to remove Paper jar")?;
	}

	Ok(())
}
