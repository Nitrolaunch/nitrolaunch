use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
	sync::Arc,
};

use nitro_pkg::{
	PkgRequest, PkgRequestSource::Repository, metadata::PackageMetadata,
	properties::PackageProperties,
};
use nitro_shared::{
	output::NitroOutput,
	pkg::{ArcPkgReq, PackageSearchParameters},
};
use reqwest::Client;
use tokio::task::JoinSet;

use crate::{io::paths::Paths, pkg::reg::PkgRegistry};

#[derive(Default, Clone)]
struct RepoState {
	/// None until we've searched this repository once.
	total_results: Option<usize>,
}

/// Session for searching multiple package repositories
pub struct PackageSearchSession {
	repos: HashMap<String, RepoState>,
	page_size: u8,
}

impl PackageSearchSession {
	/// Starts a new session
	pub fn new(repos: &[String], page_size: u8) -> Self {
		Self {
			repos: repos
				.iter()
				.cloned()
				.map(|repo| (repo, RepoState::default()))
				.collect(),
			page_size,
		}
	}

	/// Searches with the given parameters
	pub async fn search(
		&mut self,
		params: PackageSearchParameters,
		reg: Arc<PkgRegistry>,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<PackageMultiSearchResults> {
		let repo_count = self.repos.len().max(1);

		let base_share = self.page_size as usize / repo_count;
		let extra = self.page_size as usize % repo_count;

		let mut repos: Vec<_> = self.repos.keys().cloned().collect();
		repos.sort();

		let page = params.skip / self.page_size as usize;

		let mut tasks = JoinSet::new();

		for (i, repo) in repos.iter().enumerate() {
			let state = &self.repos[repo];

			let mut share = base_share;
			if (page + i) % repo_count < extra {
				share += 1;
			}

			// Slight overfetch for better blending.
			let fetch_count = share + 4;

			let repo_skip = page * share;

			if let Some(total) = state.total_results {
				if repo_skip >= total {
					continue;
				}
			}

			let mut search = params.clone();
			search.skip = repo_skip;
			search.count = fetch_count as u8;

			let reg = reg.clone();
			let paths = paths.clone();
			let client = client.clone();
			let repo = repo.clone();
			let mut output = o.get_lesser_copy();

			tasks.spawn(async move {
				let results = reg
					.search(search, Some(&repo), &paths, &client, &mut output)
					.await?;

				Ok::<_, anyhow::Error>((repo, results))
			});
		}

		struct Candidate {
			repo: String,
			id: String,
			score: u64,
		}

		let mut candidates = Vec::new();

		let mut previews = HashMap::new();
		let mut total_results = 0usize;

		while let Some(result) = tasks.join_next().await {
			let (repo, results) = result??;

			self.repos.get_mut(&repo).unwrap().total_results = Some(results.total_results);

			total_results += results.total_results;

			previews.extend(results.previews);

			for id in results.results {
				let mut hasher = std::collections::hash_map::DefaultHasher::new();

				params.search.hash(&mut hasher);
				repo.hash(&mut hasher);
				id.hash(&mut hasher);

				let score = hasher.finish();

				candidates.push(Candidate {
					repo: repo.clone(),
					id,
					score,
				});
			}
		}

		candidates.sort_by_key(|x| x.score);

		let mut output = PackageMultiSearchResults {
			total_results,
			previews,
			..Default::default()
		};

		output.results.reserve(self.page_size as usize);

		for candidate in candidates.into_iter().take(self.page_size as usize) {
			output.results.push((
				PkgRequest::parse(candidate.id, Repository).arc(),
				candidate.repo,
			));
		}

		Ok(output)
	}
}

/// Results for a package multi search
#[derive(Default, Clone)]
pub struct PackageMultiSearchResults {
	/// The package requests and repositories from the results
	pub results: Vec<(ArcPkgReq, String)>,
	/// The total number of results returned by the search, that weren't limited out
	pub total_results: usize,
	/// Limited versions of package metadata to be used for previews
	pub previews: HashMap<String, (PackageMetadata, PackageProperties)>,
}
