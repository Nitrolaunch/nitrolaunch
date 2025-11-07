use nitro_shared::UpdateDepth;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Settings for instance updates
#[derive(Debug)]
pub struct UpdateSettings {
	/// The depth to perform updates at
	pub depth: UpdateDepth,
	/// Whether to do offline authentication
	pub offline_auth: bool,
}

/// Manager for when we are updating instance files.
/// It will keep track of files we have already downloaded, manage task requirements, etc
pub struct UpdateManager {
	/// Settings for the update
	pub settings: UpdateSettings,
	/// File paths that are added when they have been updated by other functions
	files: HashSet<PathBuf>,
}

impl UpdateManager {
	/// Create a new UpdateManager
	pub fn new(depth: UpdateDepth) -> Self {
		Self::from_settings(UpdateSettings {
			depth,
			offline_auth: false,
		})
	}

	/// Create a new UpdateManager from settings
	pub fn from_settings(settings: UpdateSettings) -> Self {
		Self {
			settings,
			files: HashSet::new(),
		}
	}

	/// Add tracked files to the manager
	pub fn add_files(&mut self, files: HashSet<PathBuf>) {
		self.files.extend(files);
	}

	/// Adds an UpdateMethodResult to the UpdateManager
	pub fn add_result(&mut self, result: UpdateMethodResult) {
		self.add_files(result.files_updated);
	}

	/// Whether a file needs to be updated
	pub fn should_update_file(&self, file: &Path) -> bool {
		if self.settings.depth == UpdateDepth::Force {
			!self.files.contains(file) || !file.exists()
		} else {
			!file.exists()
		}
	}
}

/// Struct returned by updating functions, with data like changed files
#[derive(Default)]
pub struct UpdateMethodResult {
	/// The files that this function has updated
	pub files_updated: HashSet<PathBuf>,
}

impl UpdateMethodResult {
	/// Create a new UpdateMethodResult
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a new UpdateMethodResult from one path
	pub fn from_path(path: PathBuf) -> Self {
		let mut out = Self::new();
		out.files_updated.insert(path);
		out
	}

	/// Merges this result with another one
	pub fn merge(&mut self, other: Self) {
		self.files_updated.extend(other.files_updated);
	}
}
