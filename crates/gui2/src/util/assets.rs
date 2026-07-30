use std::path::PathBuf;

use freya::{
	components::{ImageSource, ImageViewer, Url},
	elements::extensions::ContainerSizeExt,
	prelude::Size,
};
use nitrolaunch::shared::{loaders::Loader, pkg::PackageKind};

pub static DEFAULT_INSTANCE: &[u8] = include_bytes!("../assets/images/default_instance.png");
pub static DEFAULT_SKIN: &[u8] = include_bytes!("../assets/images/default_skin.png");
pub static SPLASH: &[u8] = include_bytes!("../assets/images/splash.png");
pub static SPLASH2: &[u8] = include_bytes!("../assets/images/splash2.png");
pub static SPLASH3: &[u8] = include_bytes!("../assets/images/splash3.png");
pub static SPLASH4: &[u8] = include_bytes!("../assets/images/splash4.png");
pub static SPLASH5: &[u8] = include_bytes!("../assets/images/splash5.png");
pub static LOGO_LARGE: &[u8] = include_bytes!("../assets/images/logo_large.png");
pub static FABRIC: &[u8] = include_bytes!("../assets/images/fabric.png");
pub static FOLIA: &[u8] = include_bytes!("../assets/images/folia.png");
pub static FORGE: &[u8] = include_bytes!("../assets/images/forge.png");
pub static MINECRAFT: &[u8] = include_bytes!("../assets/images/minecraft.png");
pub static NEOFORGED: &[u8] = include_bytes!("../assets/images/neoforge.png");
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
				"/icons/neoforged.png" | "icons/neoforged.png" => ("neoforged", NEOFORGED).into(),
				"/icons/paper.png" => ("paper", PAPER).into(),
				"/icons/quilt.png" => ("quilt", QUILT).into(),
				"/icons/sponge.png" => ("sponge", SPONGE).into(),
				_ => default.into(),
			}
		} else if icon.starts_with("http") {
			Url::parse(icon)
				.unwrap_or(Url::parse("https://example.com").unwrap())
				.into()
		} else {
			PathBuf::from(icon).into()
		}
	} else {
		default.into()
	}
}

pub fn get_loader_icon(loader: &Loader) -> ImageViewer {
	let default = ("default-instance", DEFAULT_INSTANCE);
	let source: ImageSource = match loader {
		Loader::Vanilla => ("minecraft", MINECRAFT).into(),
		Loader::Fabric => ("fabric", FABRIC).into(),
		Loader::Folia => ("folia", FOLIA).into(),
		Loader::Forge => ("forge", FORGE).into(),
		Loader::NeoForged => ("neoforged", NEOFORGED).into(),
		Loader::Paper => ("paper", PAPER).into(),
		Loader::Quilt => ("quilt", QUILT).into(),
		Loader::Sponge | Loader::SpongeForge => ("sponge", SPONGE).into(),
		_ => default.into(),
	};
	ImageViewer::new(source)
		.width(Size::px(16.0))
		.height(Size::px(16.0))
}

pub fn get_package_kind_icon(kind: PackageKind) -> &'static str {
	match kind {
		PackageKind::Mod => "box",
		PackageKind::ResourcePack => "palette",
		PackageKind::Datapack => "curly_braces",
		PackageKind::Plugin => "jigsaw",
		PackageKind::Shader => "sun",
		PackageKind::Modpack => "honeycomb",
		PackageKind::Bundle => "folder",
	}
}
