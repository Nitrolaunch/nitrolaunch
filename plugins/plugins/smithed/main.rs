use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
};

use anyhow::Context;
use nitro_core::io::{files::create_leading_dirs, json_from_file, json_to_file};
use nitro_net::{
	download::{self, Client},
	smithed::{self, Pack, PackSearchResult},
};
use nitro_pkg::PackageSearchResults;
use nitro_pkg_gen::relation_substitution::{
	PackageAndVersion, RelationSubFunction, RelationSubNone,
};
use nitro_plugin::{
	api::{utils::PackageSearchCache, CustomPlugin},
	hooks::CustomRepoQueryResult,
};
use serde::{Deserialize, Serialize};

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("smithed", include_str!("plugin.json"))?;

	plugin.query_custom_package_repository(|ctx, arg| {
		if arg.repository != "smithed" {
			return Ok(None);
		}

		let runtime = tokio::runtime::Runtime::new()?;
		let client = Client::new();

		let data_dir = ctx.get_data_dir()?;

		runtime.block_on(query_package(&arg.package, &client, &data_dir))
	})?;

	plugin.preload_packages(|ctx, arg| {
		if arg.repository != "smithed" {
			return Ok(());
		}

		let runtime = tokio::runtime::Runtime::new()?;
		let client = Client::new();

		let data_dir = ctx.get_data_dir()?;

		runtime.block_on(async move {
			let mut tasks = tokio::task::JoinSet::new();
			for package in arg.packages {
				let client = client.clone();
				let data_dir = data_dir.clone();

				tasks.spawn(async move { query_package(&package, &client, &data_dir).await });
			}

			while let Some(task) = tasks.join_next().await {
				let _ = task??;
			}

			Ok::<(), anyhow::Error>(())
		})?;

		Ok(())
	})?;

	plugin.search_custom_package_repository(|ctx, arg| {
		if arg.repository != "smithed" {
			return Ok(PackageSearchResults::default());
		}

		let smithed_dir = ctx
			.get_data_dir()
			.context("Failed to get data dir")?
			.join("internal/smithed");

		let client = Client::new();
		let runtime = tokio::runtime::Runtime::new()?;

		let (packs, previews, total_results) = runtime.block_on(async move {
			let mut search_cache =
				PackageSearchCache::open(smithed_dir.join("search_cache.json"), 300)
					.context("Failed to open search cache")?;

			let (results, total_results) = if let Some(entry) =
				search_cache.check::<(Vec<PackSearchResult>, usize)>(&arg.parameters)
			{
				entry
			} else {
				let search_task = {
					let client = client.clone();
					let params = arg.parameters.clone();
					async move { smithed::search_packs(params, &client).await }
				};
				let count_task = {
					let client = client.clone();
					let params = arg.parameters.clone();
					async move { smithed::count_packs(params, &client).await }
				};

				let result = tokio::try_join!(search_task, count_task)?;
				let _ = search_cache.write(&arg.parameters, result.clone());
				result
			};

			let mut previews = HashMap::with_capacity(results.len());
			let mut packs = Vec::with_capacity(results.len());
			for result in results {
				packs.push(result.meta.raw_id.clone());
				let package = nitro_pkg_gen::smithed::gen(
					result.data,
					None,
					RelationSubNone,
					&[],
					Some("smithed"),
				)
				.await;
				if let Ok(package) = package {
					previews.insert(result.meta.raw_id, (package.meta, package.properties));
				}
			}

			Ok::<_, anyhow::Error>((packs, previews, total_results))
		})?;

		Ok(PackageSearchResults {
			results: packs,
			total_results,
			previews,
		})
	})?;

	plugin.sync_custom_package_repository(|ctx, arg| {
		if arg.repository != "smithed" {
			return Ok(());
		}

		let smithed_dir = ctx
			.get_data_dir()
			.context("Failed to get data dir")?
			.join("internal/smithed");

		let packs_path = smithed_dir.join("packs");
		if packs_path.exists() {
			std::fs::remove_dir_all(packs_path).context("Failed to remove cached packs")?;
		}

		Ok(())
	})?;

	Ok(())
}

/// Queries for a Smithed package
async fn query_package(
	id: &str,
	client: &Client,
	data_dir: &Path,
) -> anyhow::Result<Option<CustomRepoQueryResult>> {
	let storage_dir = data_dir.join("internal/smithed/packs");
	let pack_info = get_cached_pack(id, true, &storage_dir, &client)
		.await
		.context("Failed to get pack")?;
	let Some(pack_info) = pack_info else {
		return Ok(None);
	};

	let relation_sub_function = RelationSub {
		client: client.clone(),
		storage_dir,
	};

	let package = nitro_pkg_gen::smithed::gen(
		pack_info.pack,
		pack_info.body,
		relation_sub_function,
		&[],
		Some("smithed"),
	)
	.await
	.context("Failed to generate Nitrolaunch package")?;
	let package = serde_json::to_string_pretty(&package).context("Failed to serialized package")?;

	Ok(Some(CustomRepoQueryResult {
		contents: package,
		content_type: nitrolaunch::pkg_crate::PackageContentType::Declarative,
		flags: HashSet::new(),
	}))
}

#[derive(Clone)]
struct RelationSub {
	client: Client,
	storage_dir: PathBuf,
}

impl RelationSubFunction for RelationSub {
	async fn substitute(
		&self,
		relation: &str,
		version: Option<&str>,
	) -> anyhow::Result<PackageAndVersion> {
		let pack_info = get_cached_pack(relation, false, &self.storage_dir, &self.client)
			.await
			.context("Failed to get pack")?
			.context("Pack does not exist")?;

		let version = version
			.and_then(|version| {
				pack_info
					.pack
					.versions
					.into_iter()
					.find(|x| x.name == version)
			})
			.map(|x| x.name);

		Ok((pack_info.pack.id, version))
	}
}

/// Gets a cached Smithed pack or downloads it
async fn get_cached_pack(
	pack: &str,
	download_body: bool,
	storage_dir: &Path,
	client: &Client,
) -> anyhow::Result<Option<PackInfo>> {
	let pack_path = storage_dir.join(pack);
	// If a project does not exist, we create a dummy file so that we know not to fetch it again
	let does_not_exist_path = storage_dir.join(format!("__missing__{pack}"));
	if does_not_exist_path.exists() {
		return Ok(None);
	}

	if pack_path.exists() {
		let mut pack_info: PackInfo =
			json_from_file(&pack_path).context("Failed to read pack info from file")?;

		if download_body {
			if pack_info.body_exists && pack_info.body.is_none() {
				if let Some(body) = &pack_info.pack.display.web_page {
					if let Ok(text) = download::text(body, client).await {
						pack_info.body = Some(text);
						let _ = json_to_file(&pack_path, &pack_info);
					}
				}
			}
		}

		Ok(Some(pack_info))
	} else {
		let result = smithed::get_pack_optional(pack, &client).await?;

		let Some(pack) = result else {
			let file = std::fs::File::create(does_not_exist_path);
			std::mem::drop(file);
			return Ok(None);
		};

		let body = if download_body {
			if let Some(url) = pack.display.web_page.as_ref().filter(|x| !x.is_empty()) {
				download::text(url, client).await.ok()
			} else {
				None
			}
		} else {
			None
		};

		let pack_info = PackInfo {
			body_exists: pack.display.web_page.is_some(),
			pack,
			body,
		};

		let _ = create_leading_dirs(&pack_path);
		// TODO: Store both the id and slug together, hardlinked to each other, to cache no matter which method is used to request
		let _ = json_to_file(&pack_path, &pack_info);

		Ok(Some(pack_info))
	}
}

#[derive(Serialize, Deserialize)]
struct PackInfo {
	pack: Pack,
	body: Option<String>,
	body_exists: bool,
}
