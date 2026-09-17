use clap::Parser;
use nitro_plugin::{
	api::wasm::{
		WASMPlugin,
		sys::{get_os_string, run_command},
	},
	nitro_wasm_plugin,
};

nitro_wasm_plugin!(main, "webtools");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.subcommand(|arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "webtool" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(arg.args.into_iter().skip(1));
		let cli = Cli::try_parse_from(it)?;
		match cli.subcommand {
			Subcommand::List => list(),
			Subcommand::Open { tool } => open(&tool),
		}

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
	#[command(about = "List all available tools")]
	#[command(alias = "ls")]
	List,
	#[command(about = "Open a webtool")]
	Open {
		/// The tool to open
		tool: WebTool,
	},
}

fn list() {
	for tool in ALL_WEBTOOLS {
		println!(" - {tool} ({})", tool.name());
	}
}

fn open(webtool: &WebTool) {
	let cmd = match get_os_string().as_str() {
		"linux" => "xdg-open",
		"windows" => "start",
		"macos" => "open",
		_ => {
			println!("Link: {}", webtool.url());
			return;
		}
	};

	println!("Opening {}...", webtool.url());

	if run_command(
		cmd,
		vec![webtool.url()],
		None::<&str>,
		None::<&str>,
		true,
		true,
		false,
	)
	.is_err()
	{
		println!("Link: {}", webtool.url());
	}
}

macro_rules! define_webtools {
	($($tool:ident, $display_name:literal, $name:literal, $url:literal, $description:literal, $icon:literal, $embed_allowed:literal);+$(;)?) => {
		static ALL_WEBTOOLS: &[WebTool] = &[
			$(WebTool::$tool,)+
		];

		#[derive(Debug, Clone, Copy, clap::ValueEnum)]
		enum WebTool {
			$(
				$tool,
			)+
		}

		#[allow(dead_code)]
		impl WebTool {
			fn url(&self) -> &'static str {
				match &self {
					$(
						Self::$tool => $url,
					)+
				}
			}

			fn display_name(&self) -> &'static str {
				match &self {
					$(
						Self::$tool => $display_name,
					)+
				}
			}

			fn name(&self) -> &'static str {
				match &self {
					$(
						Self::$tool => $name,
					)+
				}
			}

			fn description(&self) -> &'static str {
				match &self {
					$(
						Self::$tool => $description,
					)+
				}
			}

			fn icon(&self) -> &'static str {
				match &self {
					$(
						Self::$tool => $icon,
					)+
				}
			}

			fn embed_allowed(&self) -> bool {
				match &self {
					$(
						Self::$tool => $embed_allowed,
					)+
				}
			}
		}

		impl std::fmt::Display for WebTool {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(f, "{}", match &self {
					$(
						Self::$tool => $display_name,
					)+
				})
			}
		}

		impl std::str::FromStr for WebTool {
			type Err = ();

			fn from_str(string: &str) -> Result<Self, Self::Err> {
				match string {
					$(
						$name => Ok(Self::$tool),
					)+
					_ => Err(()),
				}
			}
		}
	};
}

define_webtools! {
	Chunkbase, "Chunkbase", "chunkbase", "https://chunkbase.com", "Tool for mapping out Minecraft worlds", "https://chunkbase.com/favicon.ico", false;
	Wiki, "Minecraft Wiki", "wiki", "https://minecraft.wiki", "The official source for Minecraft information", "https://minecraft.wiki/images/wiki.png", false;
	McStacker, "MCStacker", "mcstacker", "https://mcstacker.net/", "Minecraft command generator", "https://mcstacker.net/favicon.ico", true;
	Misode, "Misode", "misode", "https://misode.github.io/", "Tools and generators for datapacks", "https://misode.github.io/favicon.ico", true;
}
