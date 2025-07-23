use anyhow::Context;
use clap::Parser;
use nitro_core::net::download::Client;
use nitro_plugin::api::CustomPlugin;

fn main() -> anyhow::Result<()> {
	let mut plugin = CustomPlugin::from_manifest_file("smithed_api", include_str!("plugin.json"))?;
	plugin.subcommand(|_, args| {
		let Some(subcommand) = args.first() else {
			return Ok(());
		};
		if subcommand != "smithed" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(args.into_iter().skip(1));
		let cli = Cli::parse_from(it);

		let runtime = tokio::runtime::Runtime::new()?;
		runtime.block_on(async {
			match cli.subcommand {
				Subcommand::GetPack { pack } => get_smithed_pack(pack).await,
				Subcommand::GetBundle { bundle } => get_smithed_bundle(bundle).await,
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
	#[command(about = "Get a Smithed pack")]
	GetPack {
		/// The slug or ID of the pack
		pack: String,
	},
	#[command(about = "Get a Smithed bundle")]
	GetBundle {
		/// The slug or ID of the bundle
		bundle: String,
	},
}

async fn get_smithed_pack(pack: String) -> anyhow::Result<()> {
	let client = Client::new();

	let pack = nitro_net::smithed::get_pack(&pack, &client)
		.await
		.context("Failed to get pack")?;
	let pack_pretty = serde_json::to_string_pretty(&pack)?;

	println!("{pack_pretty}");

	Ok(())
}

async fn get_smithed_bundle(bundle: String) -> anyhow::Result<()> {
	let client = Client::new();

	let bundle = nitro_net::smithed::get_bundle(&bundle, &client)
		.await
		.context("Failed to get bundle")?;
	let bundle_pretty = serde_json::to_string_pretty(&bundle)?;

	println!("{bundle_pretty}");

	Ok(())
}
