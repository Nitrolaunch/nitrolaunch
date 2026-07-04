use std::{
	collections::{BTreeMap, HashMap},
	hash::{Hash, Hasher},
	sync::Arc,
};

use dashmap::DashMap;
use nitro_pkg::{PackageMetaAndProps, PkgRequest, PkgRequestSource};
use nitro_shared::{
	output::NitroOutput,
	pkg::{ArcPkgReq, PackageSearchParameters},
};
use reqwest::Client;
use tokio::task::JoinSet;

use crate::{io::paths::Paths, pkg::reg::PkgRegistry};

#[derive(Clone, Default)]
struct RepoState {
	/// None until we've searched this repository once.
	total_results: Option<usize>,
}

/// Session for searching multiple package repositories that handles caching
#[derive(Clone)]
pub struct PackageSearchSession {
	repos: HashMap<String, RepoState>,
	page_size: u8,
	previews: Arc<DashMap<ArcPkgReq, PackageMetaAndProps>>,
	results: BTreeMap<usize, ArcPkgReq>,
	/// Used to check for search changes
	last_search: Option<PackageSearchParameters>,
	last_repo: Option<String>,
}

impl PackageSearchSession {
	/// Starts a new session
	pub fn new(page_size: u8) -> Self {
		Self {
			repos: HashMap::new(),
			page_size,
			previews: Arc::new(DashMap::new()),
			results: BTreeMap::new(),
			last_repo: None,
			last_search: None,
		}
	}

	/// Searches repositories
	pub async fn search(
		&mut self,
		params: PackageSearchParameters,
		repo: Option<&str>,
		reg: Arc<PkgRegistry>,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<PackageMultiSearchResults> {
		if let Some(repo) = repo {
			self.search_repo(params, repo, reg, paths, client, o).await
		} else {
			self.search_all(params, reg, paths, client, o).await
		}
	}

	/// Searches a single repo with the given parameters
	pub async fn search_repo(
		&mut self,
		params: PackageSearchParameters,
		repo: &str,
		reg: Arc<PkgRegistry>,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<PackageMultiSearchResults> {
		self.check_invalidate(&params, None);
		if let Some(results) = self.get_results(params.skip) {
			let total_results = self.get_total_results(Some(repo)).unwrap_or_default();
			return Ok(PackageMultiSearchResults {
				results: results.into_iter().map(|x| x.clone()).collect(),
				total_results,
			});
		}

		let skip = params.skip;
		let results = reg.search(params, repo, paths, client, o).await?;
		for preview in results.previews {
			self.previews.insert(
				PkgRequest::parse(preview.0, PkgRequestSource::Repository).arc(),
				preview.1.clone(),
			);
		}

		let reqs: Vec<_> = results
			.results
			.into_iter()
			.map(|x| PkgRequest::parse(x, PkgRequestSource::Repository).arc())
			.collect();

		for (i, req) in reqs.iter().enumerate() {
			self.results.insert(skip + i, req.clone());
		}

		Ok(PackageMultiSearchResults {
			results: reqs,
			total_results: results.total_results,
		})
	}

	/// Searches all repos with the given parameters
	pub async fn search_all(
		&mut self,
		params: PackageSearchParameters,
		reg: Arc<PkgRegistry>,
		paths: &Paths,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<PackageMultiSearchResults> {
		self.check_invalidate(&params, None);
		if let Some(results) = self.get_results(params.skip) {
			let total_results = self.get_total_results(None).unwrap_or_default();
			return Ok(PackageMultiSearchResults {
				results: results.into_iter().map(|x| x.clone()).collect(),
				total_results,
			});
		}

		let mut repos: Vec<_> = reg
			.repos
			.iter()
			.filter(|x| x.get_id() != "core")
			.map(|x| x.get_id().to_string())
			.collect();
		repos.sort();
		let repo_count = repos.len().max(1);

		let base_share = self.page_size as usize / repo_count;
		let extra = self.page_size as usize % repo_count;

		let page = params.skip / self.page_size as usize;

		let mut tasks = JoinSet::new();

		for (i, repo) in repos.iter().enumerate() {
			let state = self.repos.entry(repo.clone()).or_default();

			let mut share = base_share;
			if (page + i) % repo_count < extra {
				share += 1;
			}

			let repo_skip = page * share;

			if let Some(total) = state.total_results {
				if repo_skip >= total {
					continue;
				}
			}

			let mut search = params.clone();
			search.skip = repo_skip;
			search.count = share as u8;

			let reg = reg.clone();
			let paths = paths.clone();
			let client = client.clone();
			let repo = repo.clone();
			let mut output = o.get_lesser_copy();

			tasks.spawn(async move {
				let results = reg
					.search(search, &repo, &paths, &client, &mut output)
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

		let mut total_results = 0usize;

		while let Some(result) = tasks.join_next().await {
			let (repo, results) = result??;

			self.repos.get_mut(&repo).unwrap().total_results = Some(results.total_results);

			total_results += results.total_results;

			for preview in results.previews {
				self.previews.insert(
					PkgRequest::parse(preview.0, PkgRequestSource::Repository).arc(),
					preview.1.clone(),
				);
			}

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

		let mut out = PackageMultiSearchResults {
			total_results,
			..Default::default()
		};

		out.results.reserve(self.page_size as usize);

		for (i, candidate) in candidates
			.into_iter()
			.take(self.page_size as usize)
			.enumerate()
		{
			let mut req = PkgRequest::parse(candidate.id, PkgRequestSource::Repository);
			if req.repository.is_none() {
				req.repository = Some(candidate.repo);
			}
			let req = req.arc();
			out.results.push(req.clone());
			self.results.insert(params.skip + i, req.clone());
		}

		Ok(out)
	}

	/// Resets the results cache, while leaving previews
	pub fn reset(&mut self) {
		self.results.clear();
		self.repos.clear();
		self.last_search = None;
		self.last_repo = None;
	}

	/// Checks if search parameters have changed to determine whether to reset the cache, and does so.
	fn check_invalidate(&mut self, params: &PackageSearchParameters, repo: Option<&str>) {
		let has_changed = if let Some(current_params) = &mut self.last_search {
			repo != self.last_repo.as_deref()
				|| params.search != current_params.search
				|| params.types != current_params.types
				|| params.minecraft_versions != current_params.minecraft_versions
				|| params.loaders != current_params.loaders
				|| params.categories != current_params.categories
		} else {
			true
		};

		if has_changed {
			self.reset();
		}

		self.last_search = Some(params.clone());
		self.last_repo = repo.map(|x| x.to_string());
	}

	/// Gets a cached slice of results
	pub fn get_results(&self, start: usize) -> Option<Vec<&ArcPkgReq>> {
		let mut out = Vec::with_capacity(self.page_size as usize);
		for i in start..(start + self.page_size as usize) {
			let Some(pkg) = self.results.get(&i) else {
				return None;
			};
			out.push(pkg);
		}

		Some(out)
	}

	/// Gets the previews map
	pub fn previews(&self) -> &Arc<DashMap<ArcPkgReq, PackageMetaAndProps>> {
		&self.previews
	}

	/// Gets the total number of search results provided a search containing the given repo or all repos has already been run
	fn get_total_results(&self, repo: Option<&str>) -> Option<usize> {
		if let Some(repo) = repo {
			self.repos.get(repo).and_then(|x| x.total_results)
		} else {
			Some(
				self.repos
					.values()
					.map(|x| x.total_results.unwrap_or_default())
					.sum(),
			)
		}
	}
}

/// Results for a package multi search
#[derive(Default, Clone)]
pub struct PackageMultiSearchResults {
	/// The package results
	pub results: Vec<ArcPkgReq>,
	/// The total number of results returned by the search, that weren't limited out
	pub total_results: usize,
}
