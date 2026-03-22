use anyhow::{bail, Context};
use base64::engine::GeneralPurposeConfig;
use base64::Engine;
use nitro_plugin::api::wasm::net::download_bytes;
use nitro_plugin::api::wasm::WASMPlugin;
use nitro_plugin::nitro_wasm_plugin;
use nitro_shared::minecraft::{Cosmetic, CosmeticState, Skin, SkinVariant};
use serde::Deserialize;

nitro_wasm_plugin!(main, "skin_stealer");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.search_skin_repository(|arg| {
		if arg.repository != "steal" {
			return Ok(Vec::new());
		}

		let Some(search) = arg.search else {
			bail!("No username provided");
		};

		// Get UUID
		let result = download_bytes(&format!(
			"https://api.mojang.com/minecraft/profile/lookup/name/{search}"
		));
		let response = match result {
			Ok(response) => response,
			Err(e) => {
				let e2 = format!("{e:?}");
				if e2.contains("found") || e2.contains("404") {
					bail!("Player does not exist");
				} else {
					return Err(e);
				}
			}
		};
		let response: UUIDResponse =
			serde_json::from_slice(&response).context("Failed to deserialize UUID response")?;

		// Get skin
		let response = download_bytes(&format!(
			"https://sessionserver.mojang.com/session/minecraft/profile/{}",
			response.id
		))
		.context("Failed to download skin")?;
		let response: ProfileInfoReponse =
			serde_json::from_slice(&response).context("Failed to deserialize profile response")?;

		let base64 = base64::engine::general_purpose::GeneralPurpose::new(
			&base64::alphabet::STANDARD,
			GeneralPurposeConfig::new(),
		);

		let prop = response
			.properties
			.first()
			.context("Textures missing")?
			.value
			.clone();
		let cosmetics = base64
			.decode(prop)
			.context("Failed to decode texture data")?;
		let cosmetics: CosmeticData =
			serde_json::from_slice(&cosmetics).context("Failed to deserialize texture data")?;

		let skin = cosmetics.textures.skin;
		let variant = if skin.metadata.is_some() {
			SkinVariant::Slim
		} else {
			SkinVariant::Classic
		};

		Ok(vec![Skin {
			cosmetic: Cosmetic {
				id: skin.url.clone(),
				url: Some(skin.url),
				path: None,
				state: CosmeticState::Inactive,
			},
			variant,
		}])
	})?;

	Ok(())
}

#[derive(Deserialize)]
struct UUIDResponse {
	/// The player's UUID
	id: String,
}

#[derive(Deserialize)]
struct ProfileInfoReponse {
	properties: Vec<Property>,
}

#[derive(Deserialize)]
struct Property {
	/// Base64 CosmeticData
	value: String,
}

#[derive(Deserialize)]
struct CosmeticData {
	textures: TextureData,
}

#[derive(Deserialize)]
struct TextureData {
	/// Base64 TextureData
	#[serde(rename = "SKIN")]
	skin: SkinData,
}

#[derive(Deserialize)]
struct SkinData {
	url: String,
	#[serde(default)]
	metadata: Option<serde_json::Value>,
}
