use std::sync::Arc;

use dashmap::DashMap;
use image::{imageops, GenericImageView, RgbImage, SubImage};
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

/// Utility to resize an image to an aspect ratio, like the object-fit: cover CSS property
///
/// Takes in the ratio of width to height
pub fn crop_image_to_ratio(image: &RgbImage, ratio: f32) -> SubImage<&RgbImage> {
	let (width, height) = image.dimensions();

	let current_ratio = width as f32 / height as f32;

	if (current_ratio - ratio).abs() < f32::EPSILON {
		return image.view(0, 0, width, height);
	}

	if current_ratio > ratio {
		// Image is too wide → crop width
		let new_width = (height as f32 * ratio).round() as u32;
		let x_offset = (width - new_width) / 2;

		imageops::crop_imm(image, x_offset, 0, new_width, height)
	} else {
		// Image is too tall → crop height
		let new_height = (width as f32 / ratio).round() as u32;
		let y_offset = (height - new_height) / 2;

		imageops::crop_imm(image, 0, y_offset, width, new_height)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_crop_too_wide() {
		let image = RgbImage::new(30, 10);
		let ratio = 2.0;
		let image = crop_image_to_ratio(&image, ratio);
		assert_eq!(image.width(), 20);
		assert_eq!(image.height(), 10);
	}

	#[test]
	fn test_crop_too_tall() {
		let image = RgbImage::new(10, 30);
		let ratio = 0.5;
		let image = crop_image_to_ratio(&image, ratio);
		assert_eq!(image.width(), 10);
		assert_eq!(image.height(), 20);
	}

	#[test]
	fn test_crop_too_thin() {
		let image = RgbImage::new(10, 10);
		let ratio = 2.0;
		let image = crop_image_to_ratio(&image, ratio);
		assert_eq!(image.width(), 10);
		assert_eq!(image.height(), 5);
	}

	#[test]
	fn test_crop_too_short() {
		let image = RgbImage::new(10, 10);
		let ratio = 0.5;
		let image = crop_image_to_ratio(&image, ratio);
		assert_eq!(image.width(), 5);
		assert_eq!(image.height(), 10);
	}
}
