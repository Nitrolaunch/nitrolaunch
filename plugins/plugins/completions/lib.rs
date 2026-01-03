use clap::Parser;
use nitro_plugin::{
	api::wasm::{sys::get_plugin_dir, WASMPlugin},
	nitro_wasm_plugin,
};

nitro_wasm_plugin!(main, "completions");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.subcommand(|arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "completions" {
			return Ok(());
		}

		// Trick the parser to give it the right bin name
		let it = std::iter::once("nitro completions".into()).chain(arg.args.into_iter().skip(1));
		let cli = Cli::try_parse_from(it)?;

		match cli.command {
			Subcommand::Zsh => {
				let plugin_dir = get_plugin_dir();
				let path = plugin_dir.join("zsh").to_string_lossy().to_string();
				println!("Add the following line to your .zshrc");
				println!("if [[ $fpath[(Ie){path}] == 0 ]]; then fpath+=(\"{path}\");fi");
			}
		}

		Ok(())
	})?;

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {
	#[clap(subcommand)]
	command: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
	Zsh,
}
