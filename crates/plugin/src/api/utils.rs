use std::{
	collections::HashMap,
	fs::File,
	io::BufReader,
	path::{Path, PathBuf},
};

use anyhow::Context;
use nitro_shared::{
	pkg::PackageSearchParameters, servers::ServerSearchParams, util::utc_timestamp,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// A cache for search results in a custom plugin repository that holds entries for a certain amount of time
pub struct SearchCache {
	max_age: u64,
	contents: CacheContents,
	path: PathBuf,
}

impl SearchCache {
	/// Opens the cache at the given JSON file given the max age for an entry in seconds
	pub fn open(path: impl AsRef<Path>, max_age: u64) -> anyhow::Result<Self> {
		let contents = if path.as_ref().exists() {
			serde_json::from_reader(BufReader::new(File::open(path.as_ref())?)).unwrap_or_default()
		} else {
			let default = CacheContents::default();
			let _ = serde_json::to_writer(File::create(path.as_ref())?, &default);
			default
		};

		Ok(Self {
			max_age,
			contents,
			path: path.as_ref().to_path_buf(),
		})
	}

	/// Checks the cache for cached results
	pub fn check<S: SearchParams, R: DeserializeOwned>(&self, search: &S) -> Option<R> {
		// Don't cache searches with queries as users will want up to date results
		if search.has_search() {
			return None;
		}

		let search = serde_json::to_string(search).ok()?;
		let entry = self.contents.entries.get(&search)?;

		// Invalidate the entry if it is too old
		let timestamp = utc_timestamp().ok()?;
		if timestamp - entry.timestamp > self.max_age {
			None
		} else {
			let results = serde_json::from_value(entry.results.clone()).ok()?;
			Some(results)
		}
	}

	/// Writes to the cache
	pub fn write<S: SearchParams, R: Serialize>(
		&mut self,
		search: &S,
		results: R,
	) -> anyhow::Result<()> {
		let search = serde_json::to_string(search).context("Failed to stringify search")?;
		let results = serde_json::to_value(results).context("Failed to serialize results")?;
		let timestamp = utc_timestamp().context("Failed to get UTC timestamp")?;

		self.contents
			.entries
			.insert(search, CacheEntry { timestamp, results });

		serde_json::to_writer(File::create(&self.path)?, &self.contents)
			.context("Failed to write to cache")
	}
}

#[derive(Serialize, Deserialize, Default)]
struct CacheContents {
	entries: HashMap<String, CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
	timestamp: u64,
	results: serde_json::Value,
}

/// Trait for search parameters used in the cache
pub trait SearchParams: Serialize {
	/// Checks whether the parameters specify a fuzzy string search
	fn has_search(&self) -> bool;
}

impl SearchParams for PackageSearchParameters {
	fn has_search(&self) -> bool {
		self.search.is_some()
	}
}

impl SearchParams for ServerSearchParams {
	fn has_search(&self) -> bool {
		self.search.is_some()
	}
}
