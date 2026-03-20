use std::{
	io::{Read, Seek},
	path::Path,
};

use anyhow::Context;
use nitro_shared::Side;

/// Modrinth modpack format
pub mod mrpack;

/// A modpack that an instance can be based off of
#[async_trait::async_trait]
pub trait Modpack<R: Read + Seek + Send + 'static> {
	/// Index type for this pack containing information about its contents
	type Index;

	/// Opens this pack from a stream
	fn from_stream(r: R) -> anyhow::Result<Self>
	where
		Self: Sized;

	/// Gets the index of this pack
	fn index(&self) -> &Self::Index;

	/// Downloads all needed files for this modpack
	#[cfg(feature = "net")]
	async fn download(
		&mut self,
		addons_dir: &Path,
		client: &nitro_net::download::Client,
	) -> anyhow::Result<()>;

	/// Applies this modpack to an instance. Files must be downloaded first.
	fn apply(
		&mut self,
		target: &Path,
		addons_dir: &Path,
		side: Side,
		overwrite: bool,
	) -> anyhow::Result<()>;
}

/// Method for updating filesystem links
pub trait LinkMethod {
	/// Update a link, replacing it if it already exists
	fn link(&self, original: &Path, link: &Path) -> anyhow::Result<()>;
}

/// Default fs link method
pub struct DefaultLinkMethod;

impl LinkMethod for DefaultLinkMethod {
	fn link(&self, original: &Path, link: &Path) -> anyhow::Result<()> {
		if link.exists() {
			let _ = std::fs::remove_file(link);
		}
		#[allow(deprecated)]
		std::fs::soft_link(original, link).context("Failed to soft link")
	}
}
