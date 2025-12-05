use crate::commands::instance::InstanceOrTemplate;
use crate::{output::LauncherOutput, State};
use anyhow::Context;
use nitrolaunch::io::lock::{Lockfile, LockfilePackage};
use nitrolaunch::pkg_crate::declarative::DeclarativePackage;
use nitrolaunch::pkg_crate::metadata::PackageMetadata;
use nitrolaunch::pkg_crate::properties::PackageProperties;
use nitrolaunch::pkg_crate::repo::RepoMetadata;
use nitrolaunch::pkg_crate::{PackageSearchResults, PkgRequest, PkgRequestSource};
use nitrolaunch::shared::loaders::Loader;
use nitrolaunch::shared::output::{MessageContents, MessageLevel, NitroOutput, NoOp};
use nitrolaunch::shared::pkg::{PackageCategory, PackageKind, PackageSearchParameters};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Arc;

use super::{fmt_err, load_config};

const PACKAGES_PER_PAGE: u8 = 12;

#[tauri::command]
pub async fn get_packages(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	repo: &str,
	page: usize,
	search: Option<&str>,
	package_kinds: Vec<PackageKind>,
	minecraft_versions: Vec<String>,
	loaders: Vec<Loader>,
	categories: Vec<PackageCategory>,
) -> Result<PackageSearchResults, String> {
	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("search_packages");
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let params = PackageSearchParameters {
		count: PACKAGES_PER_PAGE,
		skip: page * PACKAGES_PER_PAGE as usize,
		search: search.map(|x| x.to_string()),
		types: package_kinds,
		minecraft_versions,
		loaders,
		categories,
	};

	let results = fmt_err(
		config
			.packages
			.search(params, Some(repo), &state.paths, &state.client, &mut output)
			.await
			.context("Failed to get list of available packages"),
	)?;

	Ok(results)
}

#[tauri::command]
pub async fn preload_packages(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	packages: Vec<String>,
	repo: Option<&str>,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let packages = packages
		.into_iter()
		.map(|x| Arc::new(PkgRequest::parse(x, PkgRequestSource::UserRequire)))
		.collect();

	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("load_packages");

	if let Some(repo) = repo {
		let repo = config
			.packages
			.repos
			.iter_mut()
			.find(|x| x.get_id() == repo);
		let Some(repo) = repo else {
			return Err("Repository does not exist".into());
		};

		fmt_err(
			repo.preload(packages, &state.paths, &config.plugins, &mut output)
				.await
				.context("Failed to preload packages from repository"),
		)?;
	} else {
		fmt_err(
			config
				.packages
				.preload_packages(packages.iter(), &state.paths, &state.client, &mut output)
				.await
				.context("Failed to preload packages from repositories"),
		)?;
	}

	Ok(())
}

#[tauri::command]
pub async fn get_package_meta(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	package: &str,
) -> Result<PackageMetadata, String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let meta = fmt_err(
		config
			.packages
			.get_metadata(
				&Arc::new(PkgRequest::parse(package, PkgRequestSource::UserRequire)),
				&state.paths,
				&state.client,
				&mut output,
			)
			.await
			.context("Failed to get metadata"),
	)?;

	Ok(meta.clone())
}

#[tauri::command]
pub async fn get_package_props(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	package: &str,
) -> Result<PackageProperties, String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let props = fmt_err(
		config
			.packages
			.get_properties(
				&Arc::new(PkgRequest::parse(package, PkgRequestSource::UserRequire)),
				&state.paths,
				&state.client,
				&mut output,
			)
			.await
			.context("Failed to get properties"),
	)?;

	Ok(props.clone())
}

#[tauri::command]
pub async fn get_package_meta_and_props(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	package: &str,
) -> Result<(PackageMetadata, PackageProperties), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let request = Arc::new(PkgRequest::parse(package, PkgRequestSource::UserRequire));

	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let meta = fmt_err(
		config
			.packages
			.get_metadata(&request, &state.paths, &state.client, &mut output)
			.await
			.context("Failed to get metadata"),
	)?
	.clone();

	let props = fmt_err(
		config
			.packages
			.get_properties(&request, &state.paths, &state.client, &mut output)
			.await
			.context("Failed to get properties"),
	)?
	.clone();

	Ok((meta, props))
}

#[tauri::command]
pub async fn get_declarative_package_contents(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
	package: &str,
) -> Result<Option<DeclarativePackage>, String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));

	let contents = fmt_err(
		config
			.packages
			.parse(
				&Arc::new(PkgRequest::parse(package, PkgRequestSource::UserRequire)),
				&state.paths,
				&state.client,
				&mut output,
			)
			.await
			.context("Failed to get properties"),
	)?;

	Ok(contents.get_declarative_contents_optional().cloned())
}

#[tauri::command]
pub async fn get_package_repos(state: tauri::State<'_, State>) -> Result<Vec<RepoInfo>, String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut repos = Vec::new();
	for repo in &mut config.packages.repos {
		let id = repo.get_id().to_string();
		let meta = fmt_err(
			repo.get_metadata(&state.paths, &state.client, &mut NoOp)
				.await
				.context("Failed to get metadata for repository"),
		)?;
		repos.push(RepoInfo {
			id,
			meta: meta.into_owned(),
		})
	}

	Ok(repos)
}

#[derive(Serialize, Deserialize)]
pub struct RepoInfo {
	pub id: String,
	pub meta: RepoMetadata,
}

#[tauri::command]
pub async fn get_instance_packages(
	state: tauri::State<'_, State>,
	instance: &str,
) -> Result<HashMap<String, LockfilePackage>, String> {
	let lock = fmt_err(Lockfile::open(&state.paths).context("Failed to open lockfile"))?;

	let default = HashMap::new();
	let packages = lock.get_instance_packages(instance).unwrap_or(&default);

	Ok(packages.clone())
}

#[tauri::command]
pub async fn sync_packages(
	state: tauri::State<'_, State>,
	app_handle: tauri::AppHandle,
) -> Result<(), String> {
	let mut config = fmt_err(
		load_config(&state.paths, &mut NoOp)
			.await
			.context("Failed to load config"),
	)?;

	let mut output = LauncherOutput::new(state.get_output(app_handle));
	output.set_task("sync_packages");

	for repo in config.packages.repos.iter_mut() {
		output.display(
			MessageContents::StartProcess(format!("Syncing repository {}", repo.get_id())),
			MessageLevel::Important,
		);
		let mut process = output.get_process();
		match repo
			.sync(
				&state.paths,
				&config.plugins,
				&state.client,
				process.deref_mut(),
			)
			.await
		{
			Ok(..) => {
				process.display(
					MessageContents::Success(format!("Synced repository {}", repo.get_id())),
					MessageLevel::Important,
				);
			}
			Err(e) => {
				process.display(
					MessageContents::Error(format!(
						"Failed to sync repository {}: {e}",
						repo.get_id()
					)),
					MessageLevel::Important,
				);
			}
		};
	}

	output.display(
		MessageContents::StartProcess("Updating standard packages".into()),
		MessageLevel::Important,
	);
	let mut process = output.get_process();
	fmt_err(
		config
			.packages
			.update_cached_packages(&state.paths, &state.client, process.deref_mut())
			.await
			.context("Failed to update cached packages"),
	)?;
	process.display(
		MessageContents::Success("Packages updated".into()),
		MessageLevel::Important,
	);

	Ok(())
}

/// Gets the last viewed package repository on the browse page
#[tauri::command]
pub async fn get_last_selected_repo(
	state: tauri::State<'_, State>,
) -> Result<Option<String>, String> {
	let data = state.data.lock().await;

	Ok(data.last_repository.clone())
}

#[tauri::command]
pub async fn set_last_selected_repo(
	state: tauri::State<'_, State>,
	repo: String,
) -> Result<(), String> {
	let mut data = state.data.lock().await;
	data.last_repository = Some(repo);

	fmt_err(data.write(&state.paths))?;

	Ok(())
}

/// Gets the instance or template where a package was last added
#[tauri::command]
pub async fn get_last_added_package_location(
	state: tauri::State<'_, State>,
) -> Result<Option<(String, InstanceOrTemplate)>, String> {
	let data = state.data.lock().await;

	Ok(data.last_added_package.clone())
}

#[tauri::command]
pub async fn set_last_added_package_location(
	state: tauri::State<'_, State>,
	id: String,
	instance_or_template: InstanceOrTemplate,
) -> Result<(), String> {
	let mut data = state.data.lock().await;
	data.last_added_package = Some((id, instance_or_template));

	fmt_err(data.write(&state.paths))?;

	Ok(())
}
