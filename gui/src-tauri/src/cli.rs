#[derive(clap::Parser)]
pub struct Cli {
	#[arg(long)]
	pub launch: Option<String>,
	#[arg(long)]
	pub account: Option<String>,
}
