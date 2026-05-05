use std::path::PathBuf;

use freya::components::{ImageSource, Uri};

pub static DEFAULT_INSTANCE: &[u8] = include_bytes!("../assets/images/default_instance.png");
pub static FABRIC: &[u8] = include_bytes!("../assets/images/fabric.png");
pub static FOLIA: &[u8] = include_bytes!("../assets/images/folia.png");
pub static FORGE: &[u8] = include_bytes!("../assets/images/forge.png");
pub static MINECRAFT: &[u8] = include_bytes!("../assets/images/minecraft.png");
pub static NEOFORGE: &[u8] = include_bytes!("../assets/images/neoforge.png");
pub static PAPER: &[u8] = include_bytes!("../assets/images/paper.png");
pub static QUILT: &[u8] = include_bytes!("../assets/images/quilt.png");
pub static SPONGE: &[u8] = include_bytes!("../assets/images/sponge.png");

pub fn get_instance_icon(icon: Option<&str>) -> ImageSource {
	let default = ("default-instance", DEFAULT_INSTANCE);

	if let Some(icon) = icon {
		if let Some(icon) = icon.strip_prefix("builtin:") {
			match icon {
				"/icons/fabric.png" => ("fabric", FABRIC).into(),
				"/icons/folia.png" => ("folia", FOLIA).into(),
				"/icons/forge.png" => ("forge", FORGE).into(),
				"/icons/minecraft.png" => ("minecraft", MINECRAFT).into(),
				"/icons/neoforge.png" => ("neoforge", NEOFORGE).into(),
				"/icons/paper.png" => ("paper", PAPER).into(),
				"/icons/quilt.png" => ("quilt", QUILT).into(),
				"/icons/sponge.png" => ("sponge", SPONGE).into(),
				_ => default.into(),
			}
		} else if icon.starts_with("http") {
			Uri::from_maybe_shared(icon.as_bytes().to_vec())
				.unwrap()
				.into()
		} else {
			PathBuf::from(icon).into()
		}
	} else {
		default.into()
	}
}
