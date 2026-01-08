use std::collections::HashSet;
use std::path::PathBuf;

use crate::{
	io::paths::Paths,
	pkg::repo::{
		basic::{BasicPackageRepository, RepoLocation},
		custom::CustomPackageRepository,
		PackageRepository,
	},
	plugin::PluginManager,
};
use nitro_config::preferences::{PrefDeser, RepoDeser};
use nitro_core::net::download::validate_url;

use anyhow::{bail, Context};
use nitro_plugin::hook::hooks::AddCustomPackageRepositories;
use nitro_shared::{
	lang::Language,
	output::{MessageContents, MessageLevel, NitroOutput},
};

/// Configured user preferences
#[derive(Debug, Default)]
pub struct ConfigPreferences {
	/// The global language
	pub language: Language,
}

impl ConfigPreferences {
	/// Convert deserialized preferences to the stored format and returns
	/// a list of repositories to add.
	pub async fn read(
		prefs: &PrefDeser,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> (Self, Vec<PackageRepository>) {
		let mut repositories = Vec::new();

		// Get repositories from plugins
		let mut preferred_plugin_repositories = Vec::new();
		let mut backup_plugin_repositories = Vec::new();
		let results = plugins
			.call_hook(AddCustomPackageRepositories, &(), paths, o)
			.await;
		match results {
			Ok(mut results) => {
				while let Some(result) = results.next() {
					let plugin_id = result.get_id().clone();
					let Ok(results) = result.result(o).await else {
						continue;
					};
					for result in results {
						let repository = PackageRepository::Custom(CustomPackageRepository::new(
							result.id,
							plugin_id.clone(),
							result.metadata,
						));
						if result.is_preferred {
							preferred_plugin_repositories.push(repository);
						} else {
							backup_plugin_repositories.push(repository);
						}
					}
				}
			}
			Err(e) => {
				o.display(
					MessageContents::Error(format!(
						"Failed to get repositories from plugins: {e:?}"
					)),
					MessageLevel::Important,
				);
			}
		}

		for repo in prefs.repositories.preferred.iter() {
			if !repo.disable {
				if let Err(e) = add_repo(&mut repositories, repo) {
					o.display(
						MessageContents::Error(format!(
							"Failed to add repository {}: {e:?}",
							repo.id
						)),
						MessageLevel::Important,
					);
				}
			}
		}
		repositories.extend(preferred_plugin_repositories);
		repositories.extend(PackageRepository::default_repos());
		repositories.extend(backup_plugin_repositories);
		for repo in prefs.repositories.backup.iter() {
			if !repo.disable {
				if let Err(e) = add_repo(&mut repositories, repo) {
					o.display(
						MessageContents::Error(format!(
							"Failed to add repository {}: {e:?}",
							repo.id
						)),
						MessageLevel::Important,
					);
				}
			}
		}

		// Check for duplicate IDs
		let mut existing = HashSet::new();
		for repo in &repositories {
			if existing.contains(&repo.get_id()) {
				o.display(
					MessageContents::Error(format!("Duplicate repository ID '{}'", repo.get_id())),
					MessageLevel::Important,
				);
			}
			existing.insert(repo.get_id());
		}

		(
			Self {
				language: prefs.language,
			},
			repositories,
		)
	}
}

/// Add a repo to the list
fn add_repo(repos: &mut Vec<PackageRepository>, repo: &RepoDeser) -> anyhow::Result<()> {
	let location = if let Some(url) = &repo.url {
		validate_url(url).with_context(|| {
			format!("Invalid url '{}' in package repository '{}'", url, repo.id)
		})?;
		RepoLocation::Remote(url.clone())
	} else if let Some(path) = &repo.path {
		RepoLocation::Local(PathBuf::from(path))
	} else {
		bail!("Niether path nor URL was set for repository {}", repo.id);
	};

	repos.push(PackageRepository::Basic(BasicPackageRepository::new(
		&repo.id, location,
	)));

	Ok(())
}
