use anyhow::Context;
use clap::Parser;
use nitro_core::net::download::Client;
use nitro_plugin::api::executable::ExecutablePlugin;

fn main() -> anyhow::Result<()> {
	let mut plugin =
		ExecutablePlugin::from_manifest_file("curseforge_api", include_str!("plugin.json"))?;
	plugin.subcommand(|_, args| {
		let Some(subcommand) = args.first() else {
			return Ok(());
		};
		if subcommand != "curse" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(args.into_iter().skip(1));
		let cli = Cli::parse_from(it);

		let runtime = tokio::runtime::Runtime::new()?;
		runtime.block_on(async {
			match cli.subcommand {
				Subcommand::GetMod { mod_id } => get_curse_mod(mod_id).await,
			}
		})?;

		Ok(())
	})?;

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
	#[command(about = "Get a CurseForge project")]
	GetMod {
		/// The slug or ID of the mod / project
		mod_id: String,
	},
}

async fn get_curse_mod(mod_id: String) -> anyhow::Result<()> {
	let client = Client::new();

	let curse_mod = nitro_net::curseforge::get_mod_raw(&mod_id, &get_api_key()?, &client)
		.await
		.context("Failed to get mod")?;
	let curse_mod_pretty = nitrolaunch::core::util::json::format_json(&curse_mod);

	let out = if let Ok(val) = curse_mod_pretty {
		val
	} else {
		curse_mod
	};

	println!("{out}");

	Ok(())
}

fn get_api_key() -> anyhow::Result<String> {
	std::env::var("NITRO_CURSEFORGE_API_KEY").context("API key missing")
}
