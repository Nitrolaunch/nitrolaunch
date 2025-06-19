use std::collections::{HashMap, HashSet};

use anyhow::Context;
use mcvm_pkg::declarative::{
	DeclarativeAddon, DeclarativeAddonVersion, DeclarativeConditionSet, DeclarativePackage,
	DeclarativePackageRelations,
};
use mcvm_pkg::metadata::PackageMetadata;
use mcvm_pkg::properties::PackageProperties;
use mcvm_pkg::RecommendedPackage;
use mcvm_shared::loaders::{Loader, LoaderMatch};
use mcvm_shared::pkg::{PackageCategory, PackageKind, PackageStability};
use mcvm_shared::util::DeserListOrSingle;
use mcvm_shared::versions::VersionPattern;

use mcvm_net::modrinth::{
	self, DependencyType, GalleryEntry, KnownLoader, License, Member, ModrinthLoader, Project,
	ProjectType, ReleaseChannel, SideSupport, Version,
};
use mcvm_shared::Side;

use crate::relation_substitution::{substitute_multiple, RelationSubFunction};

/// Generates a Modrinth package from a Modrinth project ID
pub async fn gen_from_id(
	id: &str,
	relation_substitution: impl RelationSubFunction,
	force_extensions: &[String],
	make_fabriclike: bool,
	make_forgelike: bool,
	repository: Option<&str>,
) -> anyhow::Result<DeclarativePackage> {
	let client = mcvm_core::net::download::Client::new();
	let project = modrinth::get_project(id, &client)
		.await
		.expect("Failed to get Modrinth project");

	let versions = modrinth::get_multiple_versions(&project.versions, &client)
		.await
		.expect("Failed to get Modrinth project versions");

	let members = modrinth::get_project_team(id, &client)
		.await
		.expect("Failed to get project team members from Modrinth");

	gen(
		project,
		&versions,
		&members,
		relation_substitution,
		force_extensions,
		make_fabriclike,
		make_forgelike,
		repository,
	)
	.await
}

/// Generates a Modrinth package from a Modrinth project
pub async fn gen(
	project: Project,
	versions: &[Version],
	members: &[Member],
	relation_substitution: impl RelationSubFunction,
	force_extensions: &[String],
	make_fabriclike: bool,
	make_forgelike: bool,
	repository: Option<&str>,
) -> anyhow::Result<DeclarativePackage> {
	// Get supported sides
	let supported_sides = get_supported_sides(&project);

	// Fill out metadata
	let mut meta = PackageMetadata {
		name: Some(project.title),
		description: Some(project.description),
		..Default::default()
	};
	if let Some(body) = project.body {
		meta.long_description = Some(body);
	}
	if let Some(icon_url) = project.icon_url {
		meta.icon = Some(icon_url);
	}
	if let Some(issues_url) = project.issues_url {
		meta.issues = Some(issues_url);
	}
	if let Some(source_url) = project.source_url {
		meta.source = Some(source_url);
	}
	if let Some(wiki_url) = project.wiki_url {
		meta.documentation = Some(wiki_url);
	}
	if let Some(discord_url) = project.discord_url {
		meta.community = Some(discord_url);
	}
	// Sort donation URLs as their order does not seem to be deterministic
	let mut donation_urls = project.donation_urls;
	donation_urls.sort_by_key(|x| x.url.clone());
	if let Some(support_link) = donation_urls.first() {
		meta.support_link = Some(support_link.url.clone());
	}
	if let Some(gallery) = project.gallery {
		// Get the banner image from the featured gallery image
		if let Some(banner) = gallery
			.iter()
			.find(|x| matches!(x, GalleryEntry::Full(entry) if entry.featured))
		{
			meta.banner = Some(banner.get_url().to_string());
		}
		meta.gallery = Some(
			gallery
				.into_iter()
				.map(|x| x.get_url().to_string())
				.collect(),
		);
	}

	meta.categories = Some(
		project
			.categories
			.into_iter()
			.map(|x| convert_category(&x).into_iter())
			.flatten()
			.collect(),
	);

	// Handle custom licenses
	meta.license = Some(match project.license {
		License::Short(license) => license,
		License::Long(license) => {
			if license.id == "LicenseRef-Custom" {
				if let Some(url) = license.url {
					url
				} else {
					"Custom".into()
				}
			} else {
				license.id
			}
		}
	});

	// Get team members and use them to fill out the authors field
	let mut members = members.to_vec();
	members.sort();
	meta.authors = Some(members.into_iter().map(|x| x.user.username).collect());

	// Create properties
	let mut props = PackageProperties {
		modrinth_id: Some(project.id),
		supported_sides: Some(supported_sides),
		supported_versions: Some(
			project
				.game_versions
				.into_iter()
				.map(|x| VersionPattern::from(&x))
				.collect(),
		),
		..Default::default()
	};

	// Generate addons
	let package_type = match project.project_type {
		ProjectType::Mod => PackageKind::Mod,
		ProjectType::Datapack => PackageKind::Datapack,
		ProjectType::Plugin => PackageKind::Plugin,
		ProjectType::ResourcePack => PackageKind::ResourcePack,
		ProjectType::Shader => PackageKind::Shader,
		ProjectType::Modpack => PackageKind::Bundle,
	};
	let mut addon = DeclarativeAddon {
		kind: package_type,
		versions: Vec::new(),
		conditions: Vec::new(),
		optional: false,
	};

	props.kinds = vec![package_type];

	// Make substitutions
	let mut substitutions = HashSet::new();
	for version in versions {
		for dependency in &version.dependencies {
			if let Some(project_id) = &dependency.project_id {
				substitutions.insert(project_id);
			}
		}
	}
	let substitutions = substitute_multiple(substitutions.into_iter(), relation_substitution)
		.await
		.context("Failed to substitute relations")?;

	let mut content_versions = Vec::with_capacity(versions.len());
	let mut all_loaders = HashSet::new();

	for version in versions {
		let version_name = version.id.clone();
		// Collect Minecraft versions
		let mc_versions: Vec<VersionPattern> = version
			.game_versions
			.iter()
			.map(|x| VersionPattern::Single(x.clone()))
			.collect();

		// Look at loaders
		let mut loaders = Vec::new();
		let mut skip = false;
		for loader in &version.loaders {
			match loader {
				ModrinthLoader::Known(loader) => match loader {
					KnownLoader::Fabric => loaders.push(if make_fabriclike {
						LoaderMatch::FabricLike
					} else {
						LoaderMatch::Loader(Loader::Fabric)
					}),
					KnownLoader::Quilt => loaders.push(LoaderMatch::Loader(Loader::Quilt)),
					KnownLoader::Forge => loaders.push(if make_forgelike {
						LoaderMatch::ForgeLike
					} else {
						LoaderMatch::Loader(Loader::Forge)
					}),
					KnownLoader::NeoForged => loaders.push(LoaderMatch::Loader(Loader::NeoForged)),
					KnownLoader::Rift => loaders.push(LoaderMatch::Loader(Loader::Rift)),
					KnownLoader::Liteloader => {
						loaders.push(LoaderMatch::Loader(Loader::LiteLoader))
					}
					KnownLoader::Risugamis => loaders.push(LoaderMatch::Loader(Loader::Risugamis)),
					KnownLoader::Bukkit => loaders.push(LoaderMatch::Bukkit),
					KnownLoader::Folia => loaders.push(LoaderMatch::Loader(Loader::Folia)),
					KnownLoader::Spigot => loaders.push(LoaderMatch::Loader(Loader::Spigot)),
					KnownLoader::Sponge => loaders.push(LoaderMatch::Loader(Loader::Sponge)),
					KnownLoader::Paper => loaders.push(LoaderMatch::Loader(Loader::Paper)),
					KnownLoader::Purpur => loaders.push(LoaderMatch::Loader(Loader::Purpur)),
					// Skip over these versions for now
					KnownLoader::Datapack
					| KnownLoader::BungeeCord
					| KnownLoader::Velocity
					| KnownLoader::Waterfall => skip = true,
					// We don't care about these
					KnownLoader::Iris | KnownLoader::Optifine | KnownLoader::Minecraft => {}
				},
				ModrinthLoader::Unknown(..) => {}
			}
		}
		if skip {
			continue;
		}

		all_loaders.extend(loaders.clone());
		all_loaders.extend(loaders.clone());

		// Get stability
		let stability = match version.version_type {
			ReleaseChannel::Release => PackageStability::Stable,
			ReleaseChannel::Alpha | ReleaseChannel::Beta => PackageStability::Latest,
		};

		let mut deps = Vec::new();
		let mut recommendations = Vec::new();
		let mut extensions = Vec::new();
		let mut bundled = Vec::new();
		let mut conflicts = Vec::new();

		for dep in &version.dependencies {
			let Some(project_id) = &dep.project_id else {
				continue;
			};
			let pkg_id = substitutions
				.get(project_id)
				.expect("Should have errored already")
				.clone();

			// Don't count none relations
			if pkg_id == "none" {
				continue;
			}

			let req = if let Some(repo) = &repository {
				format!("{repo}:{pkg_id}")
			} else {
				pkg_id
			};

			match dep.dependency_type {
				DependencyType::Required => {
					// Modpacks bundle all their dependencies
					if addon.kind == PackageKind::Bundle {
						bundled.push(req);
					} else if force_extensions.contains(&req) {
						extensions.push(req);
					} else {
						deps.push(req)
					}
				}
				DependencyType::Optional => recommendations.push(RecommendedPackage {
					value: req.into(),
					invert: false,
				}),
				DependencyType::Incompatible => conflicts.push(req),
				// We don't need to do anything with embedded dependencies yet
				DependencyType::Embedded => continue,
			}
		}

		// Sort relations
		deps.sort();
		recommendations.sort();
		extensions.sort();
		bundled.sort();
		conflicts.sort();

		// Content versions
		let content_version = cleanup_version_name(&version.version_number);
		if !content_versions.contains(&content_version) {
			content_versions.push(content_version.clone());
		}

		let mut pkg_version = DeclarativeAddonVersion {
			version: Some(version_name),
			conditional_properties: DeclarativeConditionSet {
				minecraft_versions: Some(DeserListOrSingle::List(mc_versions)),
				loaders: Some(DeserListOrSingle::List(loaders)),
				stability: Some(stability),
				content_versions: Some(DeserListOrSingle::Single(content_version)),
				..Default::default()
			},
			relations: DeclarativePackageRelations {
				dependencies: DeserListOrSingle::List(deps),
				recommendations: DeserListOrSingle::List(recommendations),
				extensions: DeserListOrSingle::List(extensions),
				bundled: DeserListOrSingle::List(bundled),
				conflicts: DeserListOrSingle::List(conflicts),
				..Default::default()
			},
			..Default::default()
		};

		// Select download for non-bundle kinds
		if addon.kind != PackageKind::Bundle {
			let download = version
				.get_primary_download()
				.expect("Version has no available downloads");
			pkg_version.url = Some(download.url.clone());
		}

		addon.versions.push(pkg_version);
	}

	props.content_versions = Some(content_versions);
	props.supported_loaders = Some(all_loaders.into_iter().collect());

	let mut addon_map = HashMap::new();
	addon_map.insert("addon".into(), addon);

	Ok(DeclarativePackage {
		meta,
		properties: props,
		addons: addon_map,
		..Default::default()
	})
}

/// Gets the list of supported sides from the project
fn get_supported_sides(project: &Project) -> Vec<Side> {
	let mut out = Vec::with_capacity(2);
	if let SideSupport::Required | SideSupport::Optional = &project.client_side {
		out.push(Side::Client);
	}
	if let SideSupport::Required | SideSupport::Optional = &project.server_side {
		out.push(Side::Server);
	}
	out
}

/// Cleanup a version name to remove things like loaders
fn cleanup_version_name(version: &str) -> String {
	// static MODLOADER_REGEX: OnceLock<Regex> = OnceLock::new();
	// let regex = MODLOADER_REGEX.get_or_init(|| {
	// 	RegexBuilder::new("(-|_|\\+)?(fabric|forge|quilt)")
	// 		.case_insensitive(true)
	// 		.build()
	// 		.expect("Failed to create regex")
	// });
	// let version = regex.replace_all(version, "");
	let version = version.replace("+", "-");

	version
}

fn convert_category(category: &str) -> Vec<PackageCategory> {
	match category {
		"adventure" => vec![PackageCategory::Adventure],
		"atmosphere" => vec![PackageCategory::Atmosphere],
		"audio" => vec![PackageCategory::Audio],
		"blocks" => vec![PackageCategory::Blocks, PackageCategory::Building],
		"cartoon" => vec![PackageCategory::Cartoon],
		"challenging" => vec![PackageCategory::Challenge],
		"combat" => vec![PackageCategory::Combat],
		"decoration" => vec![PackageCategory::Decoration, PackageCategory::Building],
		"economy" => vec![PackageCategory::Economy],
		"entities" => vec![PackageCategory::Entities],
		"equipment" => vec![PackageCategory::Equipment],
		"fantasy" => vec![PackageCategory::Fantasy],
		"fonts" => vec![PackageCategory::Fonts],
		"food" => vec![PackageCategory::Food],
		"game-mechanics" => vec![PackageCategory::GameMechanics],
		"gui" => vec![PackageCategory::Gui],
		"items" => vec![PackageCategory::Items],
		"kitchen-sink" => vec![PackageCategory::Extensive],
		"library" => vec![PackageCategory::Library],
		"lightweight" => vec![PackageCategory::Lightweight],
		"locale" => vec![PackageCategory::Language],
		"magic" => vec![PackageCategory::Magic],
		"minigame" => vec![PackageCategory::Minigame],
		"mobs" => vec![PackageCategory::Mobs],
		"multiplayer" => vec![PackageCategory::Multiplayer],
		"optimization" => vec![PackageCategory::Optimization],
		"realistic" => vec![PackageCategory::Realistic],
		"simplistic" => vec![PackageCategory::Simplistic],
		"social" => vec![PackageCategory::Social],
		"storage" => vec![PackageCategory::Storage],
		"technology" => vec![PackageCategory::Technology],
		"transportation" => vec![PackageCategory::Transportation],
		"tweaks" => vec![PackageCategory::Tweaks],
		"utility" => vec![PackageCategory::Utility],
		"vanilla-like" => vec![PackageCategory::VanillaPlus],
		"worldgen" => vec![PackageCategory::Worldgen, PackageCategory::Exploration],
		_ => Vec::new(),
	}
}
