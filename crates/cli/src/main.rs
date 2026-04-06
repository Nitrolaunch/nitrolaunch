/// CLI commands
mod commands;
/// NitroOutput implementation
mod output;
/// Utilities for prompting the user
mod prompt;
/// :O
mod secrets;

use std::process::ExitCode;

use commands::run_cli;

#[tokio::main]
async fn main() -> ExitCode {
	let result = run_cli().await;
	if result.is_err() {
		return ExitCode::FAILURE;
	}

	ExitCode::SUCCESS
}
