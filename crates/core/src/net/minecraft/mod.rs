use nitro_auth::mc::{call_mc_api, Keypair};
use nitro_shared::minecraft::MinecraftUserProfile;
use reqwest::Client;
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
