use anyhow::{bail, Context};
use base64::{
	engine::{GeneralPurpose, GeneralPurposeConfig},
	Engine,
};
use nitro_config::{instance::is_valid_instance_id, template::TemplateConfig};
use nitro_plugin::api::wasm::nitro::{create_template, get_templates};
use nitro_shared::id::TemplateID;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use wstd::http::{Client, Request};

/// Filename for the online bin
static FILENAME: &str = "template.json";

pub async fn export_template(template_id: &str, client: &Client) -> anyhow::Result<String> {
	let templates = get_templates().context("Failed to get templates")?;
	let Some(template) = templates.get(template_id)? else {
		bail!("Template does not exist");
	};

	// TODO: Consolidate parent templates before exporting

	let data = serde_json::to_string(&template).context("Failed to serialize template")?;

	let code = generate_code();

	upload(data, &code, FILENAME, client)
		.await
		.context("Failed to upload template")?;

	Ok(code)
}

/// Imports a template and writes the config
pub async fn import_template(template_id: &str, code: &str, client: &Client) -> anyhow::Result<()> {
	let id = TemplateID::from(template_id);

	if !is_valid_instance_id(&id) {
		bail!("Template ID is invalid");
	}

	let data = download(code, FILENAME, client)
		.await
		.context("Failed to download template. Is the code correct and still valid?")?;

	let template: TemplateConfig =
		serde_json::from_str(&data).context("Failed to deserialize template")?;

	create_template(&id, &template).context("Failed to create template")?;

	Ok(())
}

/// Generates a random code for the bucket
fn generate_code() -> String {
	let mut rng = StdRng::from_entropy();
	let base64 = GeneralPurpose::new(&base64::alphabet::URL_SAFE, GeneralPurposeConfig::new());
	const LENGTH: usize = 16;
	let mut out = [0; LENGTH];
	for i in 0..LENGTH {
		out[i] = rng.next_u64() as u8;
	}

	base64.encode(out).replace("=", "")
}

/// Uploads a file to filebin
async fn upload(
	contents: String,
	bin_id: &str,
	filename: &str,
	client: &Client,
) -> anyhow::Result<()> {
	let request = Request::post(format!("https://filebin.net/{bin_id}/{filename}"))
		.header("Content-Length", contents.as_bytes().len())
		.body(contents)?;
	let response = client.send(request).await?;
	if !response.status().is_success() {
		bail!("Error returned: {}", response.status());
	}

	Ok(())
}

/// Downloads a file from filebin.net
async fn download(bin_id: &str, filename: &str, client: &Client) -> anyhow::Result<String> {
	let request = Request::get(format!("https://filebin.net/{bin_id}/{filename}"))
		.header("Cookie", "verified=2025-05-24")
		.header("User-Agent", "curl/7.68.0")
		.body("")?;
	let mut response = client.send(request).await?;
	if !response.status().is_success() {
		bail!("Error returned: {}", response.status());
	}

	let body = response.body_mut();
	body.json().await.context("Failed to deserialize")
}
