use anyhow::{bail, Context};
use base64::{
	engine::{GeneralPurpose, GeneralPurposeConfig},
	Engine,
};
use clap::Parser;
use color_print::cprintln;
use nitro_net::{download::Client, filebin};
use nitro_plugin::api::CustomPlugin;
use nitro_shared::id::ProfileID;
use nitrolaunch::{
	config::{
		modifications::{apply_modifications_and_write, ConfigModification},
		Config,
	},
	config_crate::{instance::is_valid_instance_id, profile::ProfileConfig, ConfigDeser},
	io::paths::Paths,
};
use rand::{rngs::StdRng, RngCore, SeedableRng};

/// Filename for the online bin
static FILENAME: &str = "profile.json";

fn main() -> anyhow::Result<()> {
	let mut plugin =
		CustomPlugin::from_manifest_file("profile_share", include_str!("plugin.json"))?;
	plugin.subcommand(|_, args| {
		let Some(subcommand) = args.first() else {
			return Ok(());
		};
		let subcommand = subcommand.clone();

		// Trick the parser to give it the right bin name
		let it =
			std::iter::once(format!("nitro profile {subcommand}")).chain(args.into_iter().skip(1));

		let client = Client::new();
		let runtime = tokio::runtime::Runtime::new()?;
		let paths = Paths::new_no_create()?;

		let mut config =
			Config::open(&Config::get_path(&paths)).context("Failed to open config file")?;

		if subcommand == "share" {
			let cli = Share::parse_from(it);

			let code = export_profile(&cli.profile, &config, &runtime, &client)?;

			cprintln!("<s>Profile code: <g>{code}");
		} else if subcommand == "use" {
			let cli = Use::parse_from(it);

			import_profile(&cli.id, &cli.code, &mut config, &runtime, &paths, &client)?;

			cprintln!("<s,g>Profile added.");
		}

		Ok(())
	})?;

	plugin.custom_action(|_, arg| {
		if arg.id == "export_profile" {
			let serde_json::Value::String(id) = arg.payload else {
				bail!("Incorrect argument type");
			};

			let client = Client::new();
			let runtime = tokio::runtime::Runtime::new()?;
			let paths = Paths::new_no_create()?;

			let config =
				Config::open(&Config::get_path(&paths)).context("Failed to open config file")?;

			let code = export_profile(&id, &config, &runtime, &client)?;

			Ok(serde_json::Value::String(code))
		} else if arg.id == "import_profile" {
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

			import_profile(id, code, &mut config, &runtime, &paths, &client)?;

			Ok(serde_json::Value::Null)
		} else {
			Ok(serde_json::Value::Null)
		}
	})?;

	Ok(())
}

fn export_profile(
	profile_id: &str,
	config: &ConfigDeser,
	runtime: &tokio::runtime::Runtime,
	client: &Client,
) -> anyhow::Result<String> {
	let Some(profile) = config.profiles.get(&ProfileID::from(profile_id)) else {
		bail!("Profile does not exist");
	};

	// TODO: Consolidate parent profiles before exporting

	let data = serde_json::to_string(profile).context("Failed to serialize profile")?;

	let code = generate_code();

	runtime
		.block_on(filebin::upload(data, &code, FILENAME, &client))
		.context("Failed to upload profile")?;

	Ok(code)
}

/// Imports a profile and writes the config
fn import_profile(
	profile_id: &str,
	code: &str,
	config: &mut ConfigDeser,
	runtime: &tokio::runtime::Runtime,
	paths: &Paths,
	client: &Client,
) -> anyhow::Result<()> {
	let id = ProfileID::from(profile_id);

	if !is_valid_instance_id(&id) {
		bail!("Profile ID is invalid");
	}
	if config.profiles.contains_key(&id) {
		bail!("Profile ID '{id}' already exists. Try using another ID");
	}

	let data = runtime
		.block_on(filebin::download(code, FILENAME, &client))
		.context("Failed to download profile. Is the code correct and still valid?")?;

	let profile: ProfileConfig =
		serde_json::from_str(&data).context("Failed to deserialize profile")?;

	let modifications = vec![ConfigModification::AddProfile(id, profile)];

	apply_modifications_and_write(config, modifications, &paths)
		.context("Failed to write config")?;

	Ok(())
}

#[derive(clap::Parser)]
struct Share {
	/// The profile to share
	profile: String,
}

#[derive(clap::Parser)]
struct Use {
	/// The profile code you got from someone else
	code: String,
	/// The unique ID for the new profile
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
