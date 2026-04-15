use std::sync::Arc;

use dashmap::DashMap;
use image::RgbImage;
use nitrolaunch::net_crate::download;
use reqwest::Client;

/// Downloader and cache for images
#[derive(Clone)]
pub struct ImageCache {
	client: Client,
	cache: Arc<DashMap<String, Arc<RgbImage>>>,
}

impl ImageCache {
	/// Create a new ImageCache
	pub fn new(client: Client) -> Self {
		Self {
			client,
			cache: Arc::new(DashMap::new()),
		}
	}

	/// Gets an image from the cache or downloads it
	pub async fn get(&self, url: &str) -> anyhow::Result<Arc<RgbImage>> {
		if let Some(existing) = self.cache.get(url) {
			return Ok(existing.clone());
		}

		let data = download::bytes(url, &self.client).await?;

		let image = image::load_from_memory(&data)?;
		let image: RgbImage = image.into();
		let image = Arc::new(image);
		self.cache.insert(url.to_string(), image.clone());

		Ok(image)
	}

	/// Gets an image from the cache if it currently exists
	pub fn get_from_cache(&self, url: &str) -> Option<Arc<RgbImage>> {
		self.cache.get(url).map(|x| x.clone())
	}
}
