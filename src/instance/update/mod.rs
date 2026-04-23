/// UpdateManager
pub mod manager;
/// Modpack installation
pub mod modpack;
/// Updating packages on an instance
pub mod packages;
/// Basic setup of an instance, creating and downloading core game files
pub mod setup;

use crate::config::preferences::ConfigPreferences;
use crate::instance::update::modpack::ModpackInstallResult;
#[cfg(not(feature = "disable_instance_update_packages"))]
use crate::pkg::eval::EvalConstants;
use crate::plugin::PluginManager;
use nitro_core::NitroCore;
use nitro_core::account::AccountManager;
use nitro_pkg::{PkgRequest, PkgRequestSource};
use nitro_plugin::hook::hooks::{AfterPackagesInstalled, AfterPackagesInstalledArg};
use nitro_shared::{UpdateDepth, translate};
#[cfg(not(feature = "disable_instance_update_packages"))]
use packages::print_package_support_messages;
use packages::update_instance_packages;
#[cfg(not(feature = "disable_instance_update_packages"))]
use std::collections::HashSet;

use anyhow::Context;
use nitro_shared::output::{MessageContents, NitroOutput};
use reqwest::Client;

use crate::io::lock::Lockfile;
use crate::io::paths::Paths;
use crate::pkg::reg::PkgRegistry;

use manager::UpdateManager;

use super::Instance;

/// Shared objects for instance updating functions
pub struct InstanceUpdateContext<'a, O: NitroOutput> {
	/// The package registry
	pub packages: &'a PkgRegistry,
	/// The accounts
	pub accounts: &'a mut AccountManager,
	/// The plugins
	pub plugins: &'a PluginManager,
	/// The preferences
	pub prefs: &'a ConfigPreferences,
	/// The shared paths
	pub paths: &'a Paths,
	/// The lockfile
	pub lock: &'a mut Lockfile,
	/// The reqwest client
	pub client: &'a Client,
	/// The NitroCore
	pub core: &'a NitroCore,
	/// The output object
	pub output: &'a mut O,
}

impl Instance {
	/// Update this instance
	pub async fn update<O: NitroOutput>(
		&mut self,
		depth: UpdateDepth,
		facets: UpdateFacets,
		ctx: &mut InstanceUpdateContext<'_, O>,
	) -> anyhow::Result<()> {
		// If the instance has never been fully created, change to full update
		let has_done_first_update = ctx.lock.has_instance_done_first_update(&self.id);
		let depth = if !has_done_first_update {
			UpdateDepth::Full
		} else {
			depth
		};

		let mut manager = UpdateManager::new(depth);

		ctx.output.display(MessageContents::Header(translate!(
			ctx.output,
			StartUpdatingInstance,
			"inst" = &self.id
		)));
		ctx.output.start_section();

		let version = ctx
			.core
			.get_version(&self.version, manager.settings.depth, ctx.output)
			.await
			.context("Failed to set up core version")?;

		let version_info = version.get_version_info();
		let mc_version = version_info.version.clone();

		std::mem::drop(version);

		self.setup(
			&mut manager,
			ctx.core,
			&version_info,
			ctx.plugins,
			ctx.paths,
			ctx.output,
		)
		.await
		.context("Failed to create instance")?;

		ctx.output.end_section();

		// Modpack
		let modpack_result = if facets.modpack && depth >= UpdateDepth::Full {
			if let Some(modpack) = &self.config.modpack {
				let modpack = PkgRequest::parse(modpack, PkgRequestSource::UserRequire).arc();

				self.update_modpack(&modpack, depth, &version_info, ctx)
					.await
					.context("Failed to update modpack")?
			} else {
				ModpackInstallResult::default()
			}
		} else {
			ModpackInstallResult::default()
		};

		// Packages
		if facets.packages && depth >= UpdateDepth::Full {
			#[cfg(not(feature = "disable_instance_update_packages"))]
			{
				use std::sync::Arc;

				let mut all_packages = HashSet::new();

				ctx.output.display(MessageContents::Header(translate!(
					ctx.output,
					StartUpdatingPackages
				)));

				ctx.output.start_section();

				let constants = EvalConstants {
					version: Some(mc_version.clone()),
					loader: self.loader.clone(),
					version_list: version_info.versions.clone(),
					language: ctx.prefs.language,
					default_stability: self.config.package_stability.unwrap_or_default(),
					suppress: modpack_result.supplied_packages,
				};

				let packages = update_instance_packages(
					self,
					&Arc::new(constants),
					mc_version,
					ctx,
					depth == UpdateDepth::Force,
				)
				.await?;

				all_packages.extend(packages);

				let all_packages = Vec::from_iter(all_packages);
				let _ = print_package_support_messages(&all_packages, ctx).await;

				ctx.output.end_section();
			}
		}

		// Run hook after packages installed
		let arg = AfterPackagesInstalledArg {
			id: self.id.to_string(),
			side: Some(self.side()),
			inst_dir: self.dir.as_ref().map(|x| x.to_string_lossy().to_string()),
			version_info: version_info.clone(),
			loader: self.loader.clone(),
			config: self.config.clone(),
			internal_dir: ctx.paths.internal.to_string_lossy().to_string(),
			update_depth: manager.settings.depth,
		};

		let results = ctx
			.plugins
			.call_hook(AfterPackagesInstalled, &arg, ctx.paths, ctx.output)
			.await?;
		results.all_results(ctx.output).await?;

		ctx.lock.update_instance_has_done_first_update(&self.id);
		let _ = ctx.lock.finish(ctx.paths);

		Ok(())
	}
}

/// Parts of an instance to update
pub struct UpdateFacets {
	/// Whether to update instance files
	pub instance: bool,
	/// Whether to update packages
	pub packages: bool,
	/// Whether to update the modpack
	pub modpack: bool,
}

impl UpdateFacets {
	/// Facets with all facets enabled
	pub fn all() -> Self {
		Self {
			instance: true,
			packages: true,
			modpack: true,
		}
	}

	/// Only update packages
	pub fn packages() -> Self {
		Self {
			instance: false,
			packages: true,
			modpack: false,
		}
	}

	/// Creates facets from flags, i.e. if any of the flags are true, turns off instance updating. If all of the flags are false, sets all of them to true
	pub fn from_flags(packages: bool, modpack: bool) -> Self {
		let all_false = !packages && !modpack;
		let any_true = packages || modpack;

		let packages = if all_false { true } else { packages };
		let modpack = if all_false { true } else { modpack };

		Self {
			instance: !any_true,
			packages,
			modpack,
		}
	}
}
