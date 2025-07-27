use std::collections::{HashMap, HashSet};

use anyhow::Context;
use nitro_core::net::download::Client;
use nitro_pkg::declarative::{
	DeclarativeAddon, DeclarativeAddonVersion, DeclarativeConditionSet, DeclarativePackage,
	DeclarativePackageRelations,
};
use nitro_pkg::metadata::PackageMetadata;
use nitro_pkg::properties::PackageProperties;
use nitro_shared::pkg::{PackageCategory, PackageKind, PackageStability};
use nitro_shared::util::DeserListOrSingle;
use nitro_shared::versions::VersionPattern;

use nitro_net::smithed::Pack;

use crate::relation_substitution::{substitute_multiple, RelationSubFunction};

/// Generates a Smithed package from a Smithed pack ID
pub async fn gen_from_id(
	id: &str,
	body: Option<String>,
	relation_substitution: impl RelationSubFunction,
	force_extensions: &[String],
	repo: Option<&str>,
) -> anyhow::Result<DeclarativePackage> {
	let pack = nitro_net::smithed::get_pack(id, &Client::new())
		.await
		.expect("Failed to get pack");

	gen(pack, body, relation_substitution, force_extensions, repo).await
}

/// Generates a Smithed package from a Smithed pack
pub async fn gen(
	pack: Pack,
	body: Option<String>,
	relation_substitution: impl RelationSubFunction,
	force_extensions: &[String],
	repo: Option<&str>,
) -> anyhow::Result<DeclarativePackage> {
	let banner = if !pack.display.gallery.is_empty() {
		nitro_net::smithed::get_gallery_url(&pack.id, 0)
	} else {
		pack.display.icon.clone()
	};

	let meta = PackageMetadata {
		name: Some(pack.display.name),
		description: Some(pack.display.description),
		long_description: body,
		icon: Some(pack.display.icon),
		banner: Some(banner),
		website: pack.display.web_page,
		gallery: Some(
			std::iter::repeat(())
				.enumerate()
				.map(|(i, _)| nitro_net::smithed::get_gallery_url(&pack.id, i as u8))
				.take(pack.display.gallery.len())
				.collect(),
		),
		categories: Some(
			pack.categories
				.into_iter()
				.flat_map(|x| convert_category(&x).into_iter())
				.collect(),
		),
		..Default::default()
	};

	let mut props = PackageProperties {
		kinds: vec![PackageKind::Datapack, PackageKind::ResourcePack],
		smithed_id: Some(pack.id),
		tags: Some(vec!["datapack".into()]),
		..Default::default()
	};

	// Generate addons

	let mut datapack = DeclarativeAddon {
		kind: PackageKind::Datapack,
		versions: Vec::new(),
		conditions: Vec::new(),
		optional: false,
	};

	let mut resourcepack = DeclarativeAddon {
		kind: PackageKind::ResourcePack,
		versions: Vec::new(),
		conditions: Vec::new(),
		optional: false,
	};

	let mut all_mc_versions = Vec::new();

	let mut substitutions = HashSet::new();
	for version in &pack.versions {
		for dependency in &version.dependencies {
			substitutions.insert((dependency.id.clone(), Some(dependency.version.clone())));
		}
	}
	let substitutions = substitute_multiple(substitutions.iter(), relation_substitution)
		.await
		.context("Failed to substitute relations")?;

	for version in pack.versions.into_iter().rev() {
		// Collect Minecraft versions
		let mc_versions: Vec<VersionPattern> = version
			.supports
			.iter()
			.map(|x| VersionPattern::Single(x.clone()))
			.collect();

		// Add to all Minecraft versions
		for version in mc_versions.clone() {
			if !all_mc_versions.contains(&version) {
				all_mc_versions.push(version);
			}
		}

		let mut deps = Vec::new();
		let mut extensions = Vec::new();

		for dep in version.dependencies {
			let (dep, dep_version) = substitutions
				.get(&(dep.id.clone(), Some(dep.version.clone())))
				.expect("Should have errored already")
				.clone();

			let dep = if let Some(repo) = &repo {
				format!("{repo}:{dep}")
			} else {
				dep
			};

			let dep = if let Some(dep_version) = &dep_version {
				format!("{dep}@{dep_version}")
			} else {
				dep
			};

			if force_extensions.contains(&dep) {
				extensions.push(dep);
			} else {
				deps.push(dep);
			}
		}

		let stability = if version.name.contains("-") {
			PackageStability::Latest
		} else {
			PackageStability::Stable
		};

		let mut pkg_version = DeclarativeAddonVersion {
			version: Some(version.name.clone()),
			conditional_properties: DeclarativeConditionSet {
				minecraft_versions: Some(DeserListOrSingle::List(mc_versions)),
				content_versions: Some(DeserListOrSingle::Single(version.name)),
				stability: Some(stability),
				..Default::default()
			},
			relations: DeclarativePackageRelations {
				dependencies: DeserListOrSingle::List(deps),
				extensions: DeserListOrSingle::List(extensions),
				..Default::default()
			},
			..Default::default()
		};

		if let Some(url) = version.downloads.datapack {
			pkg_version.url = Some(url);
			datapack.versions.push(pkg_version.clone());
		}

		if let Some(url) = version.downloads.resourcepack {
			pkg_version.url = Some(url);
			resourcepack.versions.push(pkg_version.clone());
		}
	}

	props.supported_versions = Some(all_mc_versions);

	let mut addon_map = HashMap::new();
	addon_map.insert("datapack".into(), datapack);
	addon_map.insert("resourcepack".into(), resourcepack);

	Ok(DeclarativePackage {
		meta,
		properties: props,
		addons: addon_map,
		..Default::default()
	})
}

fn convert_category(category: &str) -> Vec<PackageCategory> {
	match category {
		"Extensive" => vec![PackageCategory::Extensive],
		"Lightweight" => vec![PackageCategory::Lightweight],
		"QoL" => vec![PackageCategory::Tweaks],
		"Vanilla+" => vec![PackageCategory::VanillaPlus],
		"Tech" => vec![PackageCategory::Technology],
		"Magic" => vec![PackageCategory::Magic],
		"Exploration" => vec![PackageCategory::Exploration],
		"World Overhaul" => vec![PackageCategory::Worldgen],
		"Library" => vec![PackageCategory::Library],
		_ => Vec::new(),
	}
}
