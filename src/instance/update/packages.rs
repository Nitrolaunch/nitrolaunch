use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use itertools::Itertools;
use nitro_config::instance::get_addon_paths;
use nitro_core::net::get_transfer_limit;
use nitro_pkg::overrides::is_package_overridden;
use nitro_pkg::properties::PackageProperties;
use nitro_pkg::repo::PackageFlag;
use nitro_pkg::resolve::ResolutionResult;
use nitro_pkg::PkgRequest;
use nitro_shared::addon::AddonKind;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::pkg::ArcPkgReq;
use nitro_shared::translate;
use nitro_shared::versions::VersionInfo;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::instance::Instance;
use crate::pkg::eval::{resolve, EvalConstants, EvalInput, EvalParameters};
use crate::util::select_random_n_items_from_list;

use super::InstanceUpdateContext;

use anyhow::Context;

/// Install packages on an instance. Returns a set of all unique packages
pub async fn update_instance_packages<O: NitroOutput>(
	instance: &mut Instance,
	constants: &EvalConstants,
	ctx: &mut InstanceUpdateContext<'_, O>,
	force: bool,
) -> anyhow::Result<HashSet<ArcPkgReq>> {
	// Resolve dependencies
	ctx.output.start_process();
	ctx.output.display(
		MessageContents::StartProcess(translate!(ctx.output, StartResolvingDependencies)),
		MessageLevel::Important,
	);
	let resolution = resolve_instance(instance, constants, ctx)
		.await
		.context("Failed to resolve dependencies for instance")?;
	ctx.output.display(
		MessageContents::Success(translate!(ctx.output, FinishResolvingDependencies)),
		MessageLevel::Important,
	);
	ctx.output.end_process();

	remove_existing_addons(instance, constants, ctx)?;

	// Evaluate first to install all of the addons
	ctx.output.display(
		MessageContents::StartProcess(translate!(ctx.output, StartAcquiringAddons)),
		MessageLevel::Important,
	);
	let mut tasks = HashMap::new();
	let mut evals = HashMap::new();
	for package in resolution.packages.iter().sorted_by_key(|x| x.req.clone()) {
		// Check the package to display warnings
		check_package(ctx, &package.req)
			.await
			.with_context(|| format!("Failed to check package {}", package.req))?;

		// Install the package on the instance

		// Skip suppressed packages
		if is_package_overridden(&package.req, &instance.config.package_overrides.suppress) {
			continue;
		}

		let mut params = EvalParameters::new(instance.kind.to_side());
		params.stability = instance.config.package_stability;
		if let Some(config) = instance.get_package_config(&package.req) {
			params
				.apply_config(config, &PackageProperties::default())
				.context("Failed to apply config")?;
		}

		params.required_content_versions = package.required_content_versions.clone();
		params.preferred_content_versions = package.preferred_content_versions.clone();
		params.force =
			is_package_overridden(&package.req, &instance.config.package_overrides.force);

		let input = EvalInput { constants, params };
		let (eval, new_tasks) = instance
			.get_package_addon_tasks(
				&package.req,
				input,
				ctx.packages,
				ctx.paths,
				force,
				ctx.client,
				ctx.output,
			)
			.await
			.with_context(|| {
				format!(
					"Failed to get addon install tasks for package '{}'",
					package.req
				)
			})?;
		tasks.extend(new_tasks);

		// Display any notices from the installation
		for notice in &eval.notices {
			ctx.output.display(
				format_package_update_message(
					&package.req,
					MessageContents::Notice(notice.clone()),
				),
				MessageLevel::Important,
			);
		}

		evals.insert(package.req.clone(), eval);
	}

	// Run the acquire tasks
	run_addon_tasks(tasks, ctx.output)
		.await
		.context("Failed to acquire addons")?;

	ctx.output.display(
		MessageContents::Success(translate!(ctx.output, FinishAcquiringAddons)),
		MessageLevel::Important,
	);

	// Install each package one after another onto all of its instances
	ctx.output.display(
		MessageContents::StartProcess(translate!(ctx.output, StartInstallingPackages)),
		MessageLevel::Important,
	);

	for (package, eval) in evals {
		ctx.output.start_process();

		let version_info = VersionInfo {
			version: constants.version.clone(),
			versions: constants.version_list.clone(),
		};

		instance
			.install_eval_data(
				&package,
				&eval,
				&version_info,
				ctx.paths,
				ctx.lock,
				ctx.output,
			)
			.await
			.context("Failed to install package on instance")?;

		ctx.output.display(
			format_package_update_message(
				&package,
				MessageContents::Success(translate!(ctx.output, FinishInstallingPackage)),
			),
			MessageLevel::Important,
		);
		ctx.output.end_process();
	}

	// Remove unused packages and addons
	let files_to_remove = ctx
		.lock
		.remove_unused_packages(
			&instance.id,
			&resolution
				.packages
				.iter()
				.map(|x| x.req.clone())
				.collect::<Vec<_>>(),
		)
		.context("Failed to remove unused packages")?;
	for file in files_to_remove {
		instance
			.remove_addon_file(&file, ctx.paths)
			.with_context(|| format!("Failed to remove addon file {}", file.display()))?;
	}

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

	o.start_process();
	while let Some(result) = task_set.join_next().await {
		result
			.context("Failed to run addon acquire task")?
			.context("Failed to acquire addon")?;

		// Update progress bar
		let progress = MessageContents::Progress {
			current: (total_count - task_set.len()) as u32,
			total: total_count as u32,
		};

		o.display(progress, MessageLevel::Important);
	}

	o.end_process();

	Ok(())
}

/// Resolve packages on an instance
async fn resolve_instance<O: NitroOutput>(
	instance: &mut Instance,
	constants: &EvalConstants,
	ctx: &mut InstanceUpdateContext<'_, O>,
) -> anyhow::Result<ResolutionResult> {
	let mut params = EvalParameters::new(instance.kind.to_side());
	params.stability = instance.config.package_stability;

	let instance_pkgs = instance.get_configured_packages();
	let resolution = resolve(
		instance_pkgs,
		&instance.id,
		constants,
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
fn remove_existing_addons<O: NitroOutput>(
	instance: &mut Instance,
	constants: &EvalConstants,
	ctx: &mut InstanceUpdateContext<'_, O>,
) -> anyhow::Result<()> {
	let addon_kinds = [
		AddonKind::Datapack,
		AddonKind::Mod,
		AddonKind::Plugin,
		AddonKind::ResourcePack,
		AddonKind::Shader,
	];

	for adddon_kind in addon_kinds {
		instance.ensure_dirs(ctx.paths)?;
		if instance.get_dirs().get().game_dir.is_none() {
			continue;
		}

		let Ok(dirs) = get_addon_paths(
			&instance.config.original_config_with_templates_and_plugins,
			instance
				.get_dirs()
				.get()
				.game_dir
				.as_ref()
				.expect("Game dir should exist"),
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
	let flags = ctx
		.packages
		.flags(pkg, ctx.paths, ctx.client, ctx.output)
		.await
		.context("Failed to get flags for package")?;
	if flags.contains(&PackageFlag::OutOfDate) {
		ctx.output.display(
			MessageContents::Warning(translate!(ctx.output, PackageOutOfDate, "pkg" = &pkg.id)),
			MessageLevel::Important,
		);
	}

	if flags.contains(&PackageFlag::Deprecated) {
		ctx.output.display(
			MessageContents::Warning(translate!(ctx.output, PackageDeprecated, "pkg" = &pkg.id)),
			MessageLevel::Important,
		);
	}

	if flags.contains(&PackageFlag::Insecure) {
		ctx.output.display(
			MessageContents::Error(translate!(ctx.output, PackageInsecure, "pkg" = &pkg.id)),
			MessageLevel::Important,
		);
	}

	if flags.contains(&PackageFlag::Malicious) {
		ctx.output.display(
			MessageContents::Error(translate!(ctx.output, PackageMalicious, "pkg" = &pkg.id)),
			MessageLevel::Important,
		);
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
	for package in packages {
		if let Some(link) = ctx
			.packages
			.get_metadata(package, ctx.paths, ctx.client, ctx.output)
			.await?
			.support_link
			.clone()
		{
			links.push((package, link))
		}
	}
	if !links.is_empty() {
		ctx.output.display(
			MessageContents::Header(translate!(ctx.output, PackageSupportHeader)),
			MessageLevel::Important,
		);
		for (req, link) in links {
			let msg = format_package_update_message(req, MessageContents::Hyperlink(link));
			ctx.output.display(msg, MessageLevel::Important);
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
