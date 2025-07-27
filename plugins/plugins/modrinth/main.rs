use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::Context;
use nitro_core::io::{
	files::{create_leading_dirs, update_hardlink},
	json_from_file, json_to_file,
};
use nitro_net::{
	download::Client,
	modrinth::{self, Member, Project, SearchResults, Version},
};
use nitro_pkg::{declarative::DeclarativePackage, PackageSearchResults};
use nitro_pkg_gen::{
	modrinth::{cleanup_version_name, get_preview},
	relation_substitution::{PackageAndVersion, RelationSubFunction, RelationSubNone},
};
use nitro_plugin::{
	api::{utils::PackageSearchCache, CustomPlugin},
	hooks::CustomRepoQueryResult,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("modrinth", include_str!("plugin.json"))?;

	plugin.query_custom_package_repository(|ctx, arg| {
		if arg.repository != "modrinth" {
			return Ok(None);
		}

		let data_dir = ctx.get_data_dir()?;
		let storage_dirs = StorageDirs::new(&data_dir);

		let runtime = tokio::runtime::Runtime::new()?;
		let client = Client::new();

		runtime.block_on(query_package(&arg.package, &client, &storage_dirs))
	})?;

	plugin.preload_packages(|ctx, arg| {
		if arg.repository != "modrinth" {
			return Ok(());
		}

		let data_dir = ctx.get_data_dir()?;
		let storage_dirs = StorageDirs::new(&data_dir);

		let runtime = tokio::runtime::Runtime::new()?;
		let client = Client::new();

		runtime.block_on(async move {
			if arg.packages.len() > 3 {
				let _ =
					download_multiple_projects(&arg.packages, &storage_dirs, &client, true).await;
			}

			let mut tasks = tokio::task::JoinSet::new();
			for package in arg.packages {
				let client = client.clone();
				let storage_dirs = storage_dirs.clone();

				tasks.spawn(async move { query_package(&package, &client, &storage_dirs).await });
			}

			while let Some(task) = tasks.join_next().await {
				let _ = task??;
			}

			Ok::<(), anyhow::Error>(())
		})?;

		Ok(())
	})?;

	plugin.search_custom_package_repository(|ctx, arg| {
		if arg.repository != "modrinth" {
			return Ok(PackageSearchResults::default());
		}

		let client = Client::new();
		let runtime = tokio::runtime::Runtime::new()?;

		let data_dir = ctx.get_data_dir()?;

		let (projects, previews, total_results) = runtime.block_on(async move {
			let mut search_cache =
				PackageSearchCache::open(data_dir.join("internal/modrinth/search_cache.json"), 250)
					.context("Failed to open search cache")?;

			let results =
				if let Some(results) = search_cache.check::<SearchResults>(&arg.parameters) {
					results
				} else {
					let results = modrinth::search_projects(arg.parameters.clone(), &client)
						.await
						.context("Failed to search projects from the API")?;

					let _ = search_cache.write(&arg.parameters, results.clone());
					results
				};

			let mut previews = HashMap::with_capacity(results.hits.len());
			let mut projects = Vec::with_capacity(results.hits.len());
			for result in results.hits {
				projects.push(result.slug.clone());
				let slug = result.slug.clone();
				let package = nitro_pkg_gen::modrinth::gen(
					get_preview(result),
					&[],
					&[],
					RelationSubNone,
					&[],
					true,
					true,
					Some("modrinth"),
				)
				.await;
				if let Ok(package) = package {
					previews.insert(slug, (package.meta, package.properties));
				}
			}

			Ok::<_, anyhow::Error>((projects, previews, results.total_hits))
		})?;

		Ok(PackageSearchResults {
			results: projects,
			total_results,
			previews,
		})
	})?;

	plugin.sync_custom_package_repository(|ctx, arg| {
		if arg.repository != "modrinth" {
			return Ok(());
		}

		let storage_dirs = StorageDirs::new(&ctx.get_data_dir()?);

		if storage_dirs.packages.exists() {
			std::fs::remove_dir_all(storage_dirs.packages)
				.context("Failed to remove cached packages")?;
		}
		if storage_dirs.projects.exists() {
			std::fs::remove_dir_all(storage_dirs.projects)
				.context("Failed to remove cached projects")?;
		}

		Ok(())
	})?;

	Ok(())
}

/// Queries for a Modrinth package
async fn query_package(
	id: &str,
	client: &Client,
	storage_dirs: &StorageDirs,
) -> anyhow::Result<Option<CustomRepoQueryResult>> {
	let package_or_project = get_cached_package_or_project(id, storage_dirs, client)
		.await
		.with_context(|| format!("Failed to get cached package or project '{id}'"))?;
	let Some(package_or_project) = package_or_project else {
		return Ok(None);
	};

	let package = match package_or_project {
		PackageOrProjectInfo::Package { package, .. } => package,
		PackageOrProjectInfo::ProjectInfo(project_info) => {
			let relation_sub_function = RelationSub {
				client: client.clone(),
				storage_dirs: storage_dirs.clone(),
			};

			let id = project_info.project.id.clone();
			let slug = project_info.project.slug.clone();

			let package = nitro_pkg_gen::modrinth::gen(
				project_info.project,
				&project_info.versions,
				&project_info.members,
				relation_sub_function,
				&[],
				true,
				true,
				Some("modrinth"),
			)
			.await
			.context("Failed to generate Nitrolaunch package")?;
			let package =
				serde_json::to_string_pretty(&package).context("Failed to serialized package")?;

			let package_data = format!("{id};{slug};{package}");

			let id_path = storage_dirs.packages.join(&id);
			let slug_path = storage_dirs.packages.join(&slug);
			let _ = create_leading_dirs(&id_path);
			let _ = std::fs::write(&id_path, &package_data);
			let _ = update_hardlink(&id_path, &slug_path);

			package
		}
	};

	Ok(Some(CustomRepoQueryResult {
		contents: package,
		content_type: nitrolaunch::pkg_crate::PackageContentType::Declarative,
		flags: HashSet::new(),
	}))
}

#[derive(Clone)]
struct RelationSub {
	client: Client,
	storage_dirs: StorageDirs,
}

impl RelationSubFunction for RelationSub {
	async fn substitute(
		&self,
		relation: &str,
		version: Option<&str>,
	) -> anyhow::Result<PackageAndVersion> {
		let package_or_project =
			get_cached_package_or_project(relation, &self.storage_dirs, &self.client)
				.await
				.context("Failed to get cached data")?;
		if let Some(package_or_project) = package_or_project {
			let id = match &package_or_project {
				PackageOrProjectInfo::Package { slug, .. } => slug.clone(),
				PackageOrProjectInfo::ProjectInfo(info) => info.project.slug.clone(),
			};

			let version = if let Some(version) = version {
				match package_or_project {
					PackageOrProjectInfo::Package { package, .. } => {
						let package: DeclarativePackage = serde_json::from_str(&package)?;
						package
							.addons
							.into_values()
							.find_map(|x| {
								x.versions
									.into_iter()
									.find(|x| x.version.as_ref().is_some_and(|x| x == version))
							})
							.and_then(|x| {
								x.conditional_properties
									.content_versions
									.unwrap_or_default()
									.iter()
									.next()
									.cloned()
							})
					}
					PackageOrProjectInfo::ProjectInfo(project_info) => project_info
						.versions
						.iter()
						.find(|x| x.id == version)
						.map(|x| cleanup_version_name(&x.version_number)),
				}
			} else {
				None
			};

			// Only prefer the version
			let version = version.map(|x| format!("~{x}"));

			Ok((id, version))
		} else {
			// Theres a LOT of broken Modrinth projects
			Ok(("none".into(), None))
		}
	}

	async fn preload_substitutions(&mut self, relations: &[String]) -> anyhow::Result<()> {
		// TODO: Save to internal map for quicker lookup
		if relations.len() > 5 {
			download_multiple_projects(relations, &self.storage_dirs, &self.client, false).await?;
		}
		Ok(())
	}
}

/// Gets a cached package or project info
async fn get_cached_package_or_project(
	project_id: &str,
	storage_dirs: &StorageDirs,
	client: &Client,
) -> anyhow::Result<Option<PackageOrProjectInfo>> {
	let package_path = storage_dirs.packages.join(project_id);
	// Packages are stored with their id, slug, and data separated by semicolons. This is so that we don't have to parse the full JSON.
	if package_path.exists() {
		if let Ok(data) = std::fs::read_to_string(&package_path) {
			let mut elems = data.splitn(3, ";");
			let id = elems.next().context("Missing")?;
			let slug = elems.next().context("Missing")?;
			let package = elems.next().context("Missing")?;
			// Remove the projects to save space, we don't need it anymore
			let _ = std::fs::remove_file(storage_dirs.projects.join(id));
			let _ = std::fs::remove_file(storage_dirs.projects.join(slug));
			return Ok(Some(PackageOrProjectInfo::Package {
				id: id.to_string(),
				slug: slug.to_string(),
				package: package.to_string(),
			}));
		}
	}

	get_cached_project(project_id, storage_dirs, client)
		.await
		.map(|x| x.map(PackageOrProjectInfo::ProjectInfo))
}

/// Gets a cached Modrinth project and it's versions or downloads it
async fn get_cached_project(
	project_id: &str,
	storage_dirs: &StorageDirs,
	client: &Client,
) -> anyhow::Result<Option<ProjectInfo>> {
	let project_path = storage_dirs.projects.join(project_id);
	// If a project does not exist, we create a dummy file so that we know not to fetch it again
	let does_not_exist_path = storage_dirs
		.projects
		.join(format!("__missing__{project_id}"));
	if does_not_exist_path.exists() {
		return Ok(None);
	}

	let project_info = if project_path.exists() {
		json_from_file(&project_path).context("Failed to read project info from file")?
	} else {
		let project_task = {
			let project = project_id.to_string();
			let client = client.clone();
			tokio::spawn(async move { modrinth::get_project_optional(&project, &client).await })
		};

		let members_task = {
			let project = project_id.to_string();
			let client = client.clone();
			tokio::spawn(async move { modrinth::get_project_team(&project, &client).await })
		};

		let versions_task = {
			let project = project_id.to_string();
			let client = client.clone();
			tokio::spawn(async move { modrinth::get_project_versions(&project, &client).await })
		};

		let (project, members, versions) = tokio::join!(project_task, members_task, versions_task);
		let project = project
			.context("Failed to get project")?
			.context("Failed to get project")?;
		let project = match project {
			Some(project) => project,
			None => {
				let file = std::fs::File::create(does_not_exist_path);
				std::mem::drop(file);
				return Ok(None);
			}
		};

		let members = members
			.context("Failed to get project members")?
			.context("Failed to get project members")?;
		let versions = versions
			.context("Failed to get project versions")?
			.context("Failed to get project versions")?;

		let project_info = ProjectInfo {
			project,
			versions,
			members,
		};

		let _ = save_project_info(&project_info, storage_dirs);

		project_info
	};

	Ok(Some(project_info))
}

/// Downloads multiple projects at once to save on API requests. Will have much higher latency, but is better for
/// downloading lots of projects as we won't get ratelimited
async fn download_multiple_projects(
	projects: &[String],
	storage_dirs: &StorageDirs,
	client: &Client,
	download_dependencies: bool,
) -> anyhow::Result<Vec<ProjectInfo>> {
	// Filter out projects that are already cached
	let projects: Vec<_> = projects
		.iter()
		.filter(|x| {
			let path = storage_dirs.projects.join(x);
			let path2 = storage_dirs.packages.join(x);
			!path.exists() && !path2.exists()
		})
		.cloned()
		.collect();

	if projects.is_empty() {
		return Ok(Vec::new());
	}

	let projects = modrinth::get_multiple_projects(&projects, client)
		.await
		.context("Failed to download projects")?;

	// Collect Modrinth project versions. We have to batch these into multiple requests because there becomes
	// just too many parameters for the URL to handle
	let batch_limit = 215;
	let version_ids: Vec<_> = projects
		.iter()
		.flat_map(|x| x.versions.iter().cloned())
		.collect();

	let chunks = version_ids.chunks(batch_limit);

	// Download each chunk
	let all_versions = Arc::new(Mutex::new(Vec::new()));
	let mut tasks = tokio::task::JoinSet::new();
	for chunk in chunks {
		let chunk = chunk.to_vec();
		let client = client.clone();
		let all_versions = all_versions.clone();
		let task = async move {
			let versions = modrinth::get_multiple_versions(&chunk, &client)
				.await
				.context("Failed to get Modrinth versions")?;

			let mut lock = all_versions.lock().await;
			lock.extend(versions);

			Ok::<(), anyhow::Error>(())
		};
		tasks.spawn(task);
	}

	// Download teams at the same time
	let mut team_ids = Vec::new();
	for project in &projects {
		team_ids.push(project.team.clone());
	}
	let all_teams = Arc::new(Mutex::new(Vec::new()));
	{
		let client = client.clone();
		let all_teams = all_teams.clone();
		let task = async move {
			let teams = modrinth::get_multiple_teams(&team_ids, &client)
				.await
				.context("Failed to get Modrinth teams")?;
			let mut lock = all_teams.lock().await;
			*lock = teams;

			Ok::<(), anyhow::Error>(())
		};
		tasks.spawn(task);
	}

	// Run the tasks
	while let Some(result) = tasks.join_next().await {
		result?.context("Task failed")?;
	}
	let all_versions = all_versions.lock().await;
	let all_teams = all_teams.lock().await;

	// Collect the versions into a HashMap so that we can look them up when ordering them correctly
	let mut all_versions = all_versions
		.iter()
		.map(|x| (x.id.clone(), x.clone()))
		.collect::<HashMap<_, _>>();

	let project_infos: anyhow::Result<Vec<_>> = projects
		.into_iter()
		.map(|project| {
			let versions = project
				.versions
				.iter()
				.filter_map(|x| all_versions.remove(x))
				.rev()
				.collect();

			let team = all_teams
				.iter()
				.find(|x| x.iter().any(|x| x.team_id == project.team))
				.cloned()
				.unwrap_or_default();

			Ok(ProjectInfo {
				project,
				versions,
				members: team,
			})
		})
		.collect();

	let project_infos = project_infos?;

	// Save to cache
	for project_info in &project_infos {
		let _ = save_project_info(project_info, storage_dirs);
	}

	if download_dependencies {
		let mut all_dependencies = Vec::new();
		for project in &project_infos {
			for version in &project.versions {
				for dep in &version.dependencies {
					if let Some(project_id) = &dep.project_id {
						if !all_dependencies.contains(project_id) {
							all_dependencies.push(project_id.clone());
						}
					}
				}
			}
		}

		let _ = Box::pin(download_multiple_projects(
			&all_dependencies,
			storage_dirs,
			client,
			false,
		))
		.await;
	}

	Ok(project_infos)
}

/// Saves info for a project to cache
fn save_project_info(project_info: &ProjectInfo, storage_dirs: &StorageDirs) -> anyhow::Result<()> {
	let id_path = storage_dirs.projects.join(&project_info.project.id);
	let slug_path = storage_dirs.projects.join(&project_info.project.slug);
	create_leading_dirs(&id_path)?;
	json_to_file(&id_path, &project_info)?;
	update_hardlink(&id_path, &slug_path)?;

	Ok(())
}

enum PackageOrProjectInfo {
	Package {
		#[allow(dead_code)]
		id: String,
		slug: String,
		package: String,
	},
	ProjectInfo(ProjectInfo),
}

/// Project data, versions, and team members for a Modrinth project
#[derive(Serialize, Deserialize)]
struct ProjectInfo {
	project: Project,
	versions: Vec<Version>,
	members: Vec<Member>,
}

/// Storage directories
#[derive(Clone)]
struct StorageDirs {
	projects: PathBuf,
	packages: PathBuf,
}

impl StorageDirs {
	pub fn new(data_dir: &Path) -> Self {
		let modrinth_dir = data_dir.join("internal/modrinth");
		Self {
			projects: modrinth_dir.join("projects"),
			packages: modrinth_dir.join("packages"),
		}
	}
}
