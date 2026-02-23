use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use itertools::Itertools;
use nitro_config::instance::get_addon_paths;
use nitro_core::net::get_transfer_limit;
use nitro_pkg::repo::PackageFlag;
use nitro_pkg::PkgRequest;
use nitro_shared::addon::AddonKind;
use nitro_shared::output::{MessageContents, NitroOutput};
use nitro_shared::pkg::{ArcPkgReq, PackageDiff};
use nitro_shared::translate;
use nitro_shared::versions::VersionInfo;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::instance::Instance;
use crate::pkg::eval::{resolve, EvalConstants, EvalParameters, ResolutionAndEvalResult};
use crate::util::select_random_n_items_from_list;

use super::InstanceUpdateContext;

use anyhow::{bail, Context};

/// Install packages on an instance. Returns a set of all unique packages
pub async fn update_instance_packages<O: NitroOutput>(
	instance: &mut Instance,
	constants: &Arc<EvalConstants>,
	ctx: &mut InstanceUpdateContext<'_, O>,
	force: bool,
) -> anyhow::Result<HashSet<ArcPkgReq>> {
	// Resolve dependencies
	ctx.output.start_process();
	ctx.output.display(MessageContents::StartProcess(translate!(
		ctx.output,
		StartResolvingDependencies
	)));
	let resolution = resolve_instance(instance, constants, ctx)
		.await
		.context("Failed to resolve dependencies for instance")?;
	ctx.output.display(MessageContents::Success(translate!(
		ctx.output,
		FinishResolvingDependencies
	)));
	ctx.output.end_process();

	let mut inst_lock = instance.get_lockfile(ctx.lock, ctx.paths)?;

	// Prompt to update the packages
	let current_packages = inst_lock.get_packages();
	let mut diffs = resolution.get_diffs(current_packages);

	// Make requests displayable
	for diff in &mut diffs {
		match diff {
			PackageDiff::Added(req)
			| PackageDiff::Removed(req)
			| PackageDiff::VersionChanged(req, ..) => {
				*req = ctx
					.packages
					.make_req_displayable(req, ctx.paths, ctx.client, ctx.output)
					.await
			}
		}
	}

	if !diffs.is_empty() {
		if !ctx.output.prompt_special_package_diffs(diffs).await? {
			bail!("Package update aborted");
		}
	}

	remove_existing_addons(instance, constants)?;

	// Evaluate first to install all of the addons
	ctx.output.display(MessageContents::Header(translate!(
		ctx.output,
		StartAcquiringAddons
	)));
	let mut tasks = HashMap::new();
	for package in resolution.packages.iter().sorted_by_key(|x| x.req.clone()) {
		// Check the package to display warnings
		check_package(ctx, &package.req)
			.await
			.with_context(|| format!("Failed to check package {}", package.req))?;

		// Install the package on the instance
		let new_tasks = instance
			.get_package_addon_tasks(&package.eval, ctx.paths, force, ctx.client)
			.await
			.with_context(|| {
				format!(
					"Failed to get addon install tasks for package '{}'",
					package.req
				)
			})?;
		tasks.extend(new_tasks);

		// Display any notices from the installation
		for notice in &package.eval.notices {
			ctx.output.display(format_package_update_message(
				&package.req,
				MessageContents::Notice(notice.clone()),
			));
		}
	}

	// Run the acquire tasks
	run_addon_tasks(tasks, ctx.output)
		.await
		.context("Failed to acquire addons")?;

	ctx.output.display(MessageContents::Success(translate!(
		ctx.output,
		FinishAcquiringAddons
	)));

	// Install each package one after another onto all of its instances
	ctx.output.start_process();
	ctx.output.display(MessageContents::Header(translate!(
		ctx.output,
		StartInstallingPackages
	)));

	let version_info = VersionInfo {
		version: constants.version.clone(),
		versions: constants.version_list.clone(),
	};
	for package in &resolution.packages {
		instance
			.install_eval_data(
				&package.req,
				&package.eval,
				&version_info,
				ctx.paths,
				&mut inst_lock,
				ctx.output,
			)
			.await
			.context("Failed to install package on instance")?;
	}

	// Remove unused packages and addons
	let used_package_reqs = resolution
		.packages
		.iter()
		.map(|x| x.req.clone())
		.collect::<Vec<_>>();
	let files_to_remove = inst_lock
		.remove_unused_packages(&used_package_reqs)
		.context("Failed to remove unused packages")?;
	for file in files_to_remove {
		instance
			.remove_addon_file(&file, ctx.paths)
			.with_context(|| format!("Failed to remove addon file {}", file.display()))?;
	}

	inst_lock.write()?;

	ctx.output.display(MessageContents::Success(translate!(
		ctx.output,
		FinishInstallingPackages,
		"count" = &resolution.packages.len().to_string()
	)));
	ctx.output.end_process();

	// Get the set of unique packages
	let out = HashSet::from_iter(resolution.packages.into_iter().map(|x| x.req));

	Ok(out)
}

/// Evaluates addon acquire tasks efficiently with a progress display to the user
async fn run_addon_tasks(
	tasks: HashMap<String, impl Future<Output = anyhow::Result<()>> + Send + 'static>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<()> {
	let total_count = tasks.len();
	let mut task_set = JoinSet::new();

	let sem = Arc::new(Semaphore::new(get_transfer_limit()));
	for task in tasks.into_values() {
		let permit = sem.clone().acquire_owned().await;
		let task = async move {
			let _permit = permit?;

			task.await
		};
		task_set.spawn(task);
	}

	if !task_set.is_empty() {
		let mut process = o.get_process();
		while let Some(result) = task_set.join_next().await {
			result
				.context("Failed to run addon acquire task")?
				.context("Failed to acquire addon")?;

			// Update progress bar
			let progress = MessageContents::Progress {
				current: (total_count - task_set.len()) as u32,
				total: total_count as u32,
			};

			process.display(progress);
		}
	}

	Ok(())
}

/// Resolve packages on an instance
async fn resolve_instance<O: NitroOutput>(
	instance: &mut Instance,
	constants: &Arc<EvalConstants>,
	ctx: &mut InstanceUpdateContext<'_, O>,
) -> anyhow::Result<ResolutionAndEvalResult> {
	let mut params = EvalParameters::new(instance.kind.to_side());
	params.stability = instance.config.package_stability;

	let instance_pkgs = instance.get_configured_packages();
	let resolution = resolve(
		instance_pkgs,
		&instance.id,
		constants.clone(),
		params,
		instance.config.package_overrides.clone(),
		ctx.paths,
		ctx.packages,
		ctx.client,
		ctx.output,
	)
	.await
	.with_context(|| {
		format!(
			"Failed to resolve package dependencies for instance '{}'",
			instance.id
		)
	})?;

	Ok(resolution)
}

/// Removes existing addons on an instance just in case there are lockfile issues
fn remove_existing_addons(
	instance: &mut Instance,
	constants: &EvalConstants,
) -> anyhow::Result<()> {
	let addon_kinds = [
		AddonKind::Datapack,
		AddonKind::Mod,
		AddonKind::Plugin,
		AddonKind::ResourcePack,
		AddonKind::Shader,
	];

	instance.ensure_dir()?;

	for adddon_kind in addon_kinds {
		let Some(inst_dir) = instance.get_dir() else {
			continue;
		};

		let Ok(dirs) = get_addon_paths(
			&instance.config.original_config_with_templates_and_plugins,
			inst_dir,
			adddon_kind,
			&[],
			&VersionInfo {
				version: constants.version.clone(),
				versions: constants.version_list.clone(),
			},
		) else {
			continue;
		};

		for dir in dirs {
			remove_nitro_addons(&dir);
		}
	}

	Ok(())
}

/// Checks a package with the registry to report any warnings about it
async fn check_package<O: NitroOutput>(
	ctx: &mut InstanceUpdateContext<'_, O>,
	pkg: &ArcPkgReq,
) -> anyhow::Result<()> {
	let package = ctx
		.packages
		.get(pkg, ctx.paths, ctx.client, ctx.output)
		.await?;

	if package.flags.contains(&PackageFlag::OutOfDate) {
		ctx.output.display(MessageContents::Warning(translate!(
			ctx.output,
			PackageOutOfDate,
			"pkg" = &pkg.id
		)));
	}

	if package.flags.contains(&PackageFlag::Deprecated) {
		ctx.output.display(MessageContents::Warning(translate!(
			ctx.output,
			PackageDeprecated,
			"pkg" = &pkg.id
		)));
	}

	if package.flags.contains(&PackageFlag::Insecure) {
		ctx.output.display(MessageContents::Error(translate!(
			ctx.output,
			PackageInsecure,
			"pkg" = &pkg.id
		)));
	}

	if package.flags.contains(&PackageFlag::Malicious) {
		ctx.output.display(MessageContents::Error(translate!(
			ctx.output,
			PackageMalicious,
			"pkg" = &pkg.id
		)));
	}

	Ok(())
}

/// Prints support messages about installed packages when updating
pub async fn print_package_support_messages<O: NitroOutput>(
	packages: &[ArcPkgReq],
	ctx: &mut InstanceUpdateContext<'_, O>,
) -> anyhow::Result<()> {
	let package_count = 5;
	let packages = select_random_n_items_from_list(packages, package_count);
	let mut links = Vec::new();
	for req in packages {
		let package = ctx
			.packages
			.get(req, ctx.paths, ctx.client, ctx.output)
			.await?;
		if let Some(link) = package
			.get_metadata(ctx.paths, ctx.client)
			.await?
			.support_link
			.clone()
		{
			links.push((req, link))
		}
	}
	if !links.is_empty() {
		ctx.output.display(MessageContents::Header(translate!(
			ctx.output,
			PackageSupportHeader
		)));
		for (req, link) in links {
			let msg = format_package_update_message(req, MessageContents::Hyperlink(link));
			ctx.output.display(msg);
		}
	}

	Ok(())
}

/// Creates the output message for package installation when updating an instance
fn format_package_update_message(pkg: &PkgRequest, message: MessageContents) -> MessageContents {
	MessageContents::ListItem(Box::new(MessageContents::Package(
		pkg.to_owned(),
		Box::new(message),
	)))
}

/// Removes Nitrolaunch-like addons from a directory
fn remove_nitro_addons(dir: &Path) {
	let Ok(dir) = dir.read_dir() else {
		return;
	};

	for entry in dir {
		let Ok(entry) = entry else {
			continue;
		};

		let filename = entry.file_name().to_string_lossy().to_string();
		if filename.starts_with("nitro_") && filename.contains("addon") {
			let _ = std::fs::remove_file(entry.path());
		}
	}
}
