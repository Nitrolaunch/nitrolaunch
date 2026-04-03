use std::{
	ops::DerefMut,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{bail, Context};
use nitro_instance::lock::{LockfileAddon, LockfileModpack};
use nitro_plugin::hook::hooks::{AddModpackFormats, InstallModpack, InstallModpackArg};
use nitro_shared::{
	minecraft::AddonKind,
	output::{MessageContents, NitroOutput},
	pkg::{merge_package_lists, ArcPkgReq},
	versions::VersionInfo,
	UpdateDepth,
};

use crate::{
	addon::AddonExt,
	instance::{update::InstanceUpdateContext, Instance},
	pkg::eval::{EvalConstants, EvalInput, EvalParameters, Routine},
};

impl Instance {
	/// Updates a package modpack on an instance
	pub async fn update_modpack<'a, O: NitroOutput>(
		&mut self,
		modpack: &ArcPkgReq,
		depth: UpdateDepth,
		version_info: &VersionInfo,
		ctx: &mut InstanceUpdateContext<'a, O>,
	) -> anyhow::Result<ModpackInstallResult> {
		let mut inst_lock = self.get_lockfile(ctx.paths)?;

		if self.get_dir().is_none() {
			return Ok(ModpackInstallResult::default());
		}

		// Modpack already installed
		let lock_modpack = inst_lock.get_modpack();
		if depth <= UpdateDepth::Shallow {
			if let Some(modpack) = lock_modpack {
				return Ok(ModpackInstallResult {
					supplied_packages: modpack.packages.clone(),
				});
			}
		}

		let mut process = ctx.output.get_process();
		process.display(MessageContents::StartProcess("Downloading modpack".into()));

		// Evaluate the package
		let package = ctx
			.packages
			.get(modpack, ctx.paths, ctx.client, process.deref_mut())
			.await
			.context("Failed to get modpack package")?;

		let constants = EvalConstants {
			version: version_info.version.clone(),
			loader: self.config.loader.clone(),
			version_list: version_info.versions.clone(),
			language: ctx.prefs.language,
			default_stability: self.config.package_stability,
			suppress: Vec::new(),
		};
		let params = EvalParameters::new(self.get_side());

		let input = EvalInput {
			constants: Arc::new(constants),
			params,
		};

		let result = package
			.eval(
				ctx.paths,
				Routine::Install,
				input,
				ctx.client,
				ctx.plugins.clone(),
			)
			.await
			.context("Failed to evaluate modpack")?;

		let Some(addon) = result.addon_reqs.first() else {
			process.debug(MessageContents::Simple(
				"No modpack addon was returned".into(),
			));
			process.display(MessageContents::Success("Modpack installed".into()));
			return Ok(ModpackInstallResult::default());
		};

		if addon.addon.kind != AddonKind::Modpack {
			bail!("Modpack package did not have a modpack addon");
		}

		let Some(format) = &addon.addon.modpack_format else {
			bail!("Modpack addon did not specify a format");
		};

		let bundled = result.bundled.into_iter().map(|x| x.to_string());
		let included = result.inclusions.into_iter().map(|x| x.to_string());

		// Download the modpack
		addon
			.acquire(ctx.paths, &self.id, ctx.client)
			.await
			.context("Failed to download modpack")?;
		let modpack_path = addon.addon.get_path(ctx.paths, &self.id);

		process.display(MessageContents::Success("Modpack downloaded".into()));
		process.display(MessageContents::StartProcess("Installing modpack".into()));

		let formats = ctx
			.plugins
			.call_hook(AddModpackFormats, &(), ctx.paths, process.deref_mut())
			.await?
			.flatten_all_results_with_ids(process.deref_mut())
			.await?;

		let Some((plugin_id, format)) = formats.iter().find(|x| x.1.id == *format) else {
			bail!("Modpack format {format} is not supported. Try installing a plugin for it.");
		};

		let modpack_path_str = modpack_path.to_string_lossy().to_string();

		// Don't supply the old modpack path if it is the same as the new one
		let old_path = if let Some(lock_modpack) = lock_modpack {
			if lock_modpack.path == modpack_path_str {
				None
			} else {
				if !Path::new(&lock_modpack.path).exists() {
					process.display(MessageContents::Warning(
						"Old modpack not available. Update may not work properly".into(),
					));
				}

				Some(lock_modpack.path.clone())
			}
		} else {
			None
		};

		let arg = InstallModpackArg {
			format: format.id.clone(),
			path: modpack_path_str.clone(),
			old_path,
			target_path: self.get_dir().unwrap().to_string_lossy().to_string(),
			side: self.get_side(),
		};

		let result = ctx
			.plugins
			.call_hook_on_plugin(
				InstallModpack,
				plugin_id,
				&arg,
				ctx.paths,
				process.deref_mut(),
			)
			.await?;

		let result = result.context("Modpack install was not handled by plugin")?;
		let result = result
			.result(process.deref_mut())
			.await
			.context("Failed to install modpack")?;

		let addons: Vec<_> = result
			.addons
			.into_iter()
			.map(|x| LockfileAddon {
				id: None,
				package: None,
				from_modpack: true,
				file_name: x.file_name,
				files: x
					.target_paths
					.into_iter()
					.map(|x| x.to_string_lossy().to_string())
					.collect(),
				kind: x.kind,
				hashes: x.hashes,
			})
			.collect();

		// Combine bundled and included dependencies from the package with the results from the modpack
		let packages = result.packages;
		let packages = merge_package_lists(bundled, &packages);
		let packages = merge_package_lists(included, &packages);

		let lockfile_modpack = LockfileModpack {
			name: result.name,
			path: modpack_path_str,
			packages,
		};

		let files_to_remove = inst_lock.update_modpack(lockfile_modpack, &addons);
		for file in files_to_remove {
			let file = PathBuf::from(file);
			if file.exists() {
				std::fs::remove_file(file)?;
			}
		}
		inst_lock.write().context("Failed to write lockfile")?;

		process.display(MessageContents::Success("Modpack installed".into()));

		Ok(ModpackInstallResult::default())
	}
}

/// Result from updating modpack installation
#[derive(Default, Clone)]
pub struct ModpackInstallResult {
	/// Packages provided by the modpack
	pub supplied_packages: Vec<String>,
}
