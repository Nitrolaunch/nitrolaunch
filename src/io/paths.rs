use anyhow::Context;

use std::path::PathBuf;

/// Store for all of the paths that are used throughout the application
#[derive(Debug, Clone)]
pub struct Paths {
	/// Paths object from core
	pub core: nitro_core::Paths,
	/// Config directory
	pub config: PathBuf,
	/// Holds program data
	pub data: PathBuf,
	/// Holds internal data
	pub internal: PathBuf,
	/// Holds addons
	pub addons: PathBuf,
	/// Holds cached package scripts
	pub pkg_cache: PathBuf,
	/// Holds cached package repository indexes
	pub pkg_index_cache: PathBuf,
	/// Holds log files
	pub logs: PathBuf,
	/// Holding user plugins
	pub plugins: PathBuf,
}

impl Paths {
	/// Create a new Paths object and also create all of the paths it contains on the filesystem
	pub async fn new() -> anyhow::Result<Paths> {
		let out = Self::new_no_create()?;
		out.create_dirs().await?;

		Ok(out)
	}

	/// Create the directories on an existing set of paths
	pub async fn create_dirs(&self) -> anyhow::Result<()> {
		let _ = tokio::join!(
			tokio::fs::create_dir_all(&self.addons),
			tokio::fs::create_dir_all(&self.pkg_cache),
			tokio::fs::create_dir_all(&self.pkg_index_cache),
			tokio::fs::create_dir_all(&self.logs),
			tokio::fs::create_dir_all(&self.plugins),
		);

		self.core.create_dirs()?;

		Ok(())
	}

	/// Create the paths without creating any directories
	pub fn new_no_create() -> anyhow::Result<Self> {
		let core_paths = nitro_core::Paths::new().context("Failed to create core paths")?;
		let data = core_paths.data.clone();

		let internal = data.join("internal");
		let addons = internal.join("addons");
		let pkg_cache = internal.join("pkg");
		let pkg_index_cache = pkg_cache.join("index");
		let logs = data.join("logs");
		let plugins = data.join("plugins");

		Ok(Paths {
			config: core_paths.config.clone(),
			data,
			core: core_paths,
			internal,
			addons,
			pkg_cache,
			pkg_index_cache,
			logs,
			plugins,
		})
	}
}
