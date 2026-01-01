use anyhow::{bail, Context};
use base64::{
	engine::{GeneralPurpose, GeneralPurposeConfig},
	Engine,
};
use clap::Parser;
use color_print::cprintln;
use nitro_net::{download::Client, filebin};
use nitro_plugin::api::executable::ExecutablePlugin;
use nitro_shared::{id::TemplateID, output::NoOp};
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::{instance::is_valid_instance_id, template::TemplateConfig, ConfigDeser},
	io::paths::Paths,
	plugin::PluginManager,
};
use rand::{rngs::StdRng, RngCore, SeedableRng};

/// Filename for the online bin
static FILENAME: &str = "template.json";

fn main() -> anyhow::Result<()> {
	let mut plugin =
		ExecutablePlugin::from_manifest_file("template_share", include_str!("plugin.json"))?;
	plugin.subcommand(|_, arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		let subcommand = subcommand.clone();

		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro template {subcommand}"))
			.chain(arg.args.into_iter().skip(1));

		let client = Client::new();
		let runtime = tokio::runtime::Runtime::new()?;
		let paths = Paths::new_no_create()?;

		let mut config =
			Config::open(&Config::get_path(&paths)).context("Failed to open config file")?;

		if subcommand == "share" {
			let cli = Share::parse_from(it);

			let code = export_template(&cli.template, &config, &runtime, &client)?;

			cprintln!("<s>Template code: <g>{code}");
		} else if subcommand == "use" {
			let cli = Use::parse_from(it);

			import_template(&cli.id, &cli.code, &mut config, &runtime, &paths, &client)?;

			cprintln!("<s,g>Template added.");
		}

		Ok(())
	})?;

	plugin.custom_action(|_, arg| {
		if arg.id == "export_template" {
			let serde_json::Value::String(id) = arg.payload else {
				bail!("Incorrect argument type");
			};

			let client = Client::new();
			let runtime = tokio::runtime::Runtime::new()?;
			let paths = Paths::new_no_create()?;

			let config =
				Config::open(&Config::get_path(&paths)).context("Failed to open config file")?;

			let code = export_template(&id, &config, &runtime, &client)?;

			Ok(serde_json::Value::String(code))
		} else if arg.id == "import_template" {
			let serde_json::Value::Object(map) = arg.payload else {
				bail!("Incorrect argument type");
			};

			let Some(serde_json::Value::String(id)) = map.get("id") else {
				bail!("Incorrect argument type");
			};

			let Some(serde_json::Value::String(code)) = map.get("code") else {
				bail!("Incorrect argument type");
			};

			let client = Client::new();
			let runtime = tokio::runtime::Runtime::new()?;
			let paths = Paths::new_no_create()?;

			let mut config =
				Config::open(&Config::get_path(&paths)).context("Failed to open config file")?;

			import_template(id, code, &mut config, &runtime, &paths, &client)?;

			Ok(serde_json::Value::Null)
		} else {
			Ok(serde_json::Value::Null)
		}
	})?;

	Ok(())
}

fn export_template(
	template_id: &str,
	config: &ConfigDeser,
	runtime: &tokio::runtime::Runtime,
	client: &Client,
) -> anyhow::Result<String> {
	let Some(template) = config.templates.get(&TemplateID::from(template_id)) else {
		bail!("Template does not exist");
	};

	// TODO: Consolidate parent templates before exporting

	let data = serde_json::to_string(template).context("Failed to serialize template")?;

	let code = generate_code();

	runtime
		.block_on(filebin::upload(data, &code, FILENAME, client))
		.context("Failed to upload template")?;

	Ok(code)
}

/// Imports a template and writes the config
fn import_template(
	template_id: &str,
	code: &str,
	config: &mut ConfigDeser,
	runtime: &tokio::runtime::Runtime,
	paths: &Paths,
	client: &Client,
) -> anyhow::Result<()> {
	let id = TemplateID::from(template_id);

	if !is_valid_instance_id(&id) {
		bail!("Template ID is invalid");
	}
	if config.templates.contains_key(&id) {
		bail!("Template ID '{id}' already exists. Try using another ID");
	}

	let plugins = runtime.block_on(PluginManager::load(paths, &mut NoOp))?;

	let data = runtime
		.block_on(filebin::download(code, FILENAME, client))
		.context("Failed to download template. Is the code correct and still valid?")?;

	let template: TemplateConfig =
		serde_json::from_str(&data).context("Failed to deserialize template")?;

	let modifications = vec![ConfigModification::AddTemplate(id, template)];

	runtime
		.block_on(apply_modifications_and_write(
			config,
			modifications,
			paths,
			&plugins,
		))
		.context("Failed to write config")?;

	Ok(())
}

#[derive(clap::Parser)]
struct Share {
	/// The template to share
	template: String,
}

#[derive(clap::Parser)]
struct Use {
	/// The template code you got from someone else
	code: String,
	/// The unique ID for the new template
	id: String,
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
