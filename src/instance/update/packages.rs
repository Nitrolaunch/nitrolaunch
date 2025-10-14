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
use nitro_pkg::PkgRequest;
use nitro_shared::addon::AddonKind;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::pkg::{ArcPkgReq, PackageID};
use nitro_shared::translate;
use nitro_shared::versions::VersionInfo;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::instance::Instance;
use crate::pkg::eval::{resolve, EvalConstants, EvalInput, EvalParameters};
use crate::util::select_random_n_items_from_list;
use nitro_shared::id::InstanceID;

use super::InstanceUpdateContext;

use anyhow::Context;

/// Install packages on multiple instances. Returns a set of all unique packages
pub async fn update_instance_packages<'a, O: NitroOutput>(
	instances: &mut [&mut Instance],
	constants: &EvalConstants,
	ctx: &mut InstanceUpdateContext<'a, O>,
	force: bool,
) -> anyhow::Result<HashSet<ArcPkgReq>> {
	// Resolve dependencies
	ctx.output.start_process();
	ctx.output.display(
		MessageContents::StartProcess(translate!(ctx.output, StartResolvingDependencies)),
		MessageLevel::Important,
	);
	let resolved_packages = resolve_and_batch(instances, constants, ctx)
		.await
		.context("Failed to resolve dependencies for profile")?;
	ctx.output.display(
		MessageContents::Success(translate!(ctx.output, FinishResolvingDependencies)),
		MessageLevel::Important,
	);
	ctx.output.end_process();

	// Blanket remove existing addons just in case there's a lockfile issue
	let addon_kinds = [
		AddonKind::Datapack,
		AddonKind::Mod,
		AddonKind::Plugin,
		AddonKind::ResourcePack,
		AddonKind::Shader,
	];

	for adddon_kind in addon_kinds {
		for instance in instances.iter_mut() {
			instance.ensure_dirs(ctx.paths)?;

			let Ok(dirs) = get_addon_paths(
				&instance.config.original_config_with_profiles_and_plugins,
				&instance.get_dirs().get().game_dir,
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
	}

	// Evaluate first to install all of the addons
	ctx.output.display(
		MessageContents::StartProcess(translate!(ctx.output, StartAcquiringAddons)),
		MessageLevel::Important,
	);
	let mut tasks = HashMap::new();
	let mut evals = HashMap::new();
	for (package, package_instances) in resolved_packages
		.package_to_instances
		.iter()
		.sorted_by_key(|x| x.0)
	{
		// Check the package to display warnings
		check_package(ctx, package)
			.await
			.with_context(|| format!("Failed to check package {package}"))?;

		// Install the package on it's instances
		let mut notices = Vec::new();
		for instance_id in package_instances {
			let instance = instances
				.iter_mut()
				.find(|x| &x.id == instance_id)
				.expect("Instance should exist");

			// Skip suppressed packages
			if is_package_overridden(package, &instance.config.package_overrides.suppress) {
				continue;
			}

			let mut params = EvalParameters::new(instance.kind.to_side());
			params.stability = instance.config.package_stability;
			if let Some(config) = instance.get_package_config(&package.to_string()) {
				params
					.apply_config(config, &PackageProperties::default())
					.context("Failed to apply config")?;
			}

			let default = (Vec::new(), Vec::new());
			let content_version_params = resolved_packages
				.content_version_params
				.get(package)
				.unwrap_or(&default);
			params.required_content_versions = content_version_params.0.clone();
			params.preferred_content_versions = content_version_params.1.clone();

			let input = EvalInput { constants, params };
			let (eval, new_tasks) = instance
				.get_package_addon_tasks(
					package,
					input,
					ctx.packages,
					ctx.paths,
					force,
					ctx.client,
					ctx.output,
				)
				.await
				.with_context(|| {
					format!("Failed to get addon install tasks for package '{package}' on instance")
				})?;
			tasks.extend(new_tasks);

			// Add any notices to the list
			notices.extend(
				eval.notices
					.iter()
					.map(|x| (instance_id.clone(), x.to_owned())),
			);

			// Add the eval to the map
			evals.insert((package, instance_id), eval);
		}

		// Display any accumulated notices from the installation
		for (instance, notice) in notices {
			ctx.output.display(
				format_package_update_message(
					package,
					Some(&instance),
					MessageContents::Notice(notice),
				),
				MessageLevel::Important,
			);
		}
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
	for (package, package_instances) in resolved_packages
		.package_to_instances
		.iter()
		.sorted_by_key(|x| x.0)
	{
		ctx.output.start_process();

		for instance_id in package_instances {
			let instance = instances
				.iter_mut()
				.find(|x| &x.id == instance_id)
				.expect("Instance should exist");

			let version_info = VersionInfo {
				version: constants.version.clone(),
				versions: constants.version_list.clone(),
			};
			let Some(eval) = evals.get(&(package, instance_id)) else {
				// Suppressed packages won't be in the map
				continue;
			};

			instance
				.install_eval_data(
					package,
					eval,
					&version_info,
					ctx.paths,
					ctx.lock,
					ctx.output,
				)
				.await
				.context("Failed to install package on instance")?;
		}

		ctx.output.display(
			format_package_update_message(
				package,
				None,
				MessageContents::Success(translate!(ctx.output, FinishInstallingPackage)),
			),
			MessageLevel::Important,
		);
		ctx.output.end_process();
	}

	// Use the instance-package map to remove unused packages and addons
	for (instance_id, packages) in resolved_packages.instance_to_packages {
		let instance = instances
			.iter()
			.find(|x| x.id == instance_id)
			.expect("Instance should exist");

		let files_to_remove = ctx
			.lock
			.remove_unused_packages(
				&instance_id,
				&packages
					.iter()
					.map(|x| x.id.clone())
					.collect::<Vec<PackageID>>(),
			)
			.context("Failed to remove unused packages")?;
		for file in files_to_remove {
			instance
				.remove_addon_file(&file, ctx.paths)
				.with_context(|| {
					format!(
						"Failed to remove addon file {} for instance {}",
						file.display(),
						instance_id
					)
				})?;
		}
	}

	// Get the set of unique packages
	let mut out = HashSet::new();
	out.extend(resolved_packages.package_to_instances.keys().cloned());

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

/// Resolve packages and create a mapping of packages to a list of instances.
/// This allows us to update packages in a reasonable order to the user.
/// It also returns a map of instances to packages so that unused packages can be removed
async fn resolve_and_batch<'a, O: NitroOutput>(
	instances: &[&mut Instance],
	constants: &EvalConstants,
	ctx: &mut InstanceUpdateContext<'a, O>,
) -> anyhow::Result<ResolvedPackages> {
	let mut batched: HashMap<ArcPkgReq, Vec<InstanceID>> = HashMap::new();
	let mut resolved = HashMap::new();
	let mut content_version_params = HashMap::new();

	for instance in instances {
		let mut params = EvalParameters::new(instance.kind.to_side());
		params.stability = instance.config.package_stability;

		let instance_pkgs = instance.get_configured_packages();
		let instance_resolved = resolve(
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

		for result in &instance_resolved.packages {
			if let Some(entry) = batched.get_mut(&result.req) {
				entry.push(instance.id.clone());
			} else {
				batched.insert(result.req.clone(), vec![instance.id.clone()]);
			}

			content_version_params.insert(
				result.req.clone(),
				(
					result.required_content_versions.clone(),
					result.preferred_content_versions.clone(),
				),
			);
		}
		resolved.insert(
			instance.id.clone(),
			instance_resolved
				.packages
				.into_iter()
				.map(|x| x.req)
				.collect(),
		);
	}

	Ok(ResolvedPackages {
		package_to_instances: batched,
		instance_to_packages: resolved,
		content_version_params,
	})
}

struct ResolvedPackages {
	/// A mapping of package IDs to all of the instances they are installed on
	pub package_to_instances: HashMap<ArcPkgReq, Vec<InstanceID>>,
	/// A reverse mapping of instance IDs to all of the packages they have resolved
	pub instance_to_packages: HashMap<InstanceID, Vec<ArcPkgReq>>,
	/// A mapping of packages to their content version parameters (required and preferred content versions)
	pub content_version_params: HashMap<ArcPkgReq, (Vec<String>, Vec<String>)>,
}

/// Checks a package with the registry to report any warnings about it
async fn check_package<'a, O: NitroOutput>(
	ctx: &mut InstanceUpdateContext<'a, O>,
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
pub async fn print_package_support_messages<'a, O: NitroOutput>(
	packages: &[ArcPkgReq],
	ctx: &mut InstanceUpdateContext<'a, O>,
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
			let msg = format_package_update_message(req, None, MessageContents::Hyperlink(link));
			ctx.output.display(msg, MessageLevel::Important);
		}
	}

	Ok(())
}

/// Creates the output message for package installation when updating profiles
fn format_package_update_message(
	pkg: &PkgRequest,
	instance: Option<&str>,
	message: MessageContents,
) -> MessageContents {
	let msg = if let Some(instance) = instance {
		MessageContents::Package(
			pkg.to_owned(),
			Box::new(MessageContents::Associated(
				Box::new(MessageContents::Simple(instance.to_string())),
				Box::new(message),
			)),
		)
	} else {
		MessageContents::Package(pkg.to_owned(), Box::new(message))
	};

	MessageContents::ListItem(Box::new(msg))
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
