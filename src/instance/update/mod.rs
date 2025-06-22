/// UpdateManager
pub mod manager;
/// Updating packages on a profile
pub mod packages;

use crate::config::preferences::ConfigPreferences;
#[cfg(not(feature = "disable_profile_update_packages"))]
use crate::pkg::eval::EvalConstants;
use crate::plugin::PluginManager;
use mcvm_core::user::UserManager;
use mcvm_shared::{translate, UpdateDepth};
#[cfg(not(feature = "disable_profile_update_packages"))]
use packages::print_package_support_messages;
use packages::update_instance_packages;
#[cfg(not(feature = "disable_profile_update_packages"))]
use std::collections::HashSet;

use anyhow::Context;
use mcvm_shared::output::{MCVMOutput, MessageContents, MessageLevel};
use reqwest::Client;

use crate::io::lock::Lockfile;
use crate::io::paths::Paths;
use crate::pkg::reg::PkgRegistry;

use manager::UpdateManager;

use super::Instance;

/// Shared objects for instance updating functions
pub struct InstanceUpdateContext<'a, O: MCVMOutput> {
	/// The package registry
	pub packages: &'a mut PkgRegistry,
	/// The users
	pub users: &'a UserManager,
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
	/// The output object
	pub output: &'a mut O,
}

impl Instance {
	/// Update this instance
	pub async fn update<'a, O: MCVMOutput>(
		&mut self,
		update_packages: bool,
		depth: UpdateDepth,
		ctx: &mut InstanceUpdateContext<'a, O>,
	) -> anyhow::Result<()> {
		#[cfg(feature = "disable_profile_update_packages")]
		let _update_packages = update_packages;

		let mut manager = UpdateManager::new(depth);

		ctx.output.display(
			MessageContents::Header(translate!(
				ctx.output,
				StartUpdatingInstance,
				"inst" = &self.id
			)),
			MessageLevel::Important,
		);

		manager.set_version(&self.config.version);
		manager.add_requirements(self.get_requirements());
		manager
			.fulfill_requirements(ctx.users, ctx.plugins, ctx.paths, ctx.client, ctx.output)
			.await
			.context("Failed to fulfill update manager")?;
		let mc_version = manager.version_info.get().version.clone();

		ctx.lock
			.finish(ctx.paths)
			.context("Failed to finish using lockfile")?;

		self.setup(
			&mut manager,
			ctx.plugins,
			ctx.paths,
			ctx.users,
			ctx.lock,
			ctx.output,
		)
		.await
		.context("Failed to create instance")?;

		if update_packages {
			#[cfg(not(feature = "disable_profile_update_packages"))]
			{
				let mut all_packages = HashSet::new();

				ctx.output.display(
					MessageContents::Header(translate!(ctx.output, StartUpdatingPackages)),
					MessageLevel::Important,
				);

				let constants = EvalConstants {
					version: mc_version.to_string(),
					loader: self.config.loader.clone(),
					version_list: manager.version_info.get().versions.clone(),
					language: ctx.prefs.language,
					profile_stability: self.config.package_stability,
				};

				let packages = update_instance_packages(
					&mut [self],
					&constants,
					ctx,
					depth == UpdateDepth::Force,
				)
				.await?;

				ctx.output.display(
					MessageContents::Success(translate!(ctx.output, FinishUpdatingPackages)),
					MessageLevel::Important,
				);

				all_packages.extend(packages);

				ctx.lock
					.finish(ctx.paths)
					.context("Failed to finish using lockfile")?;

				let all_packages = Vec::from_iter(all_packages);
				print_package_support_messages(&all_packages, ctx)
					.await
					.context("Failed to print support messages")?;
			}
		}

		Ok(())
	}
}
