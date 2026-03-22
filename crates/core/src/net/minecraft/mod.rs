use nitro_auth::mc::{call_mc_api, Keypair};
use nitro_shared::minecraft::{MinecraftUserProfile, SkinVariant};
use reqwest::{
	multipart::{Form, Part},
	Client,
};
use serde::Deserialize;

/// Get a Minecraft user profile
pub async fn get_user_profile(
	access_token: &str,
	client: &Client,
) -> anyhow::Result<MinecraftUserProfile> {
	call_mc_api(
		"https://api.minecraftservices.com/minecraft/profile",
		access_token,
		client,
	)
	.await
}

/// Response from the player certificate endpoint
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftUserCertificate {
	/// Public / private key pair
	pub key_pair: Keypair,
}

/// Get a Minecraft user certificate
pub async fn get_user_certificate(
	access_token: &str,
	client: &Client,
) -> anyhow::Result<MinecraftUserCertificate> {
	let response = client
		.post("https://api.minecraftservices.com/player/certificates")
		.header("Authorization", format!("Bearer {access_token}"))
		.send()
		.await?
		.error_for_status()?
		.json()
		.await?;

	Ok(response)
}

/// Uploads a skin
pub async fn upload_skin(
	variant: SkinVariant,
	skin: &[u8],
	access_token: &str,
	client: &Client,
) -> anyhow::Result<()> {
	let variant = match variant {
		SkinVariant::Classic => "classic",
		SkinVariant::Slim => "slim",
	};
	let form = Form::new().text("variant", variant).part(
		"file",
		Part::bytes(skin.to_vec())
			.file_name("skin.png")
			.mime_str("image/png")?,
	);

	client
		.post("https://api.minecraftservices.com/minecraft/profile/skins")
		.header("Authorization", format!("Bearer {access_token}"))
		.multipart(form)
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}
