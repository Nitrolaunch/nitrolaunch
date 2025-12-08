use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use nitro_auth::mc::AccessToken;
use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput};
use nitro_shared::versions::VersionName;
use nitro_shared::{no_window, translate};

use crate::instance::InstanceKind;
use crate::io::files::open_file_append;
use crate::{InstanceHandle, Paths, WrapperCommand};

use super::LaunchConfiguration;

/// Launch the game process
pub(crate) fn launch_game_process(
	mut params: LaunchGameProcessParameters<'_>,
	o: &mut impl NitroOutput,
) -> anyhow::Result<InstanceHandle> {
	// Modify the parameters based on game-specific properties

	// Prepend generated game args to the beginning
	let previous_game_args = params.props.game_args.clone();
	params.props.game_args = params.launch_config.generate_game_args(
		params.version,
		params.version_list,
		params.side.get_side(),
		o,
	);
	params.props.game_args.extend(previous_game_args);

	// Create the parameters for the process
	let proc_params = LaunchProcessParameters {
		command: params.command,
		cwd: params.cwd,
		main_class: params.main_class,
		props: params.props,
		launch_config: params.launch_config,
	};

	o.display(
		MessageContents::Success(translate!(o, Launch)),
		MessageLevel::Important,
	);

	// Stdio files
	let stdout = get_stdio_file_path(params.paths, false);
	let stdin = if params.pipe_stdin {
		None
	} else {
		Some(get_stdio_file_path(params.paths, true))
	};

	// Get the command and output it
	let mut cmd = get_process_launch_command(proc_params, &stdout, stdin.as_deref())
		.context("Failed to create process launch command")?;

	output_launch_command(&cmd, params.user_access_token, params.censor_secrets, o)?;

	// Spawn
	let child = cmd.spawn().context("Failed to spawn child process")?;

	let stdout_file = File::open(&stdout)?;
	let stdin_file = if let Some(stdin) = &stdin {
		Some(open_file_append(stdin)?)
	} else {
		None
	};

	Ok(InstanceHandle::new(
		child,
		stdout_file,
		stdout,
		stdin_file,
		stdin,
	))
}

/// Launch a generic process with the core's config system
pub fn launch_process(
	params: LaunchProcessParameters<'_>,
	stdout_path: &Path,
	stdin_path: Option<&Path>,
) -> anyhow::Result<Child> {
	let mut cmd = get_process_launch_command(params, stdout_path, stdin_path)
		.context("Failed to create process launch command")?;

	cmd.spawn().context("Failed to spawn child process")
}

/// Get the command for launching a generic process using the core's config system
pub fn get_process_launch_command(
	params: LaunchProcessParameters<'_>,
	stdout_path: &Path,
	stdin_path: Option<&Path>,
) -> anyhow::Result<Command> {
	// Create the base command based on wrapper settings
	let mut cmd = create_wrapped_command(params.command, &params.launch_config.wrappers);

	// Fill out the command properties
	cmd.current_dir(params.cwd);
	cmd.envs(params.launch_config.env.clone());
	cmd.envs(params.props.additional_env_vars);

	// Add the arguments
	cmd.args(params.launch_config.generate_jvm_args());
	cmd.args(params.props.jvm_args);
	if let Some(main_class) = params.main_class {
		cmd.arg(main_class);
	}
	cmd.args(params.props.game_args);

	// Capture stdio
	let stdout = File::create_new(stdout_path).context("Failed to open stdout")?;
	cmd.stdout(std::process::Stdio::from(stdout));
	if let Some(stdin_path) = stdin_path {
		let stdin = File::create_new(stdin_path).context("Failed to open stdin")?;
		cmd.stdin(std::process::Stdio::from(stdin));
	} else {
		cmd.stdin(std::process::Stdio::inherit());
	}

	no_window!(cmd);

	Ok(cmd)
}

/// Display the launch command in our own way,
/// censoring any credentials if needed
fn output_launch_command(
	command: &Command,
	access_token: Option<&AccessToken>,
	censor_secrets: bool,
	o: &mut impl NitroOutput,
) -> anyhow::Result<()> {
	o.end_process();
	let access_token = if censor_secrets { access_token } else { None };
	o.display(
		MessageContents::Property(
			"Launch command".into(),
			Box::new(MessageContents::Simple(
				command.get_program().to_string_lossy().into(),
			)),
		),
		MessageLevel::Debug,
	);

	o.display(
		MessageContents::Header("Launch command arguments".into()),
		MessageLevel::Debug,
	);

	const CENSOR_STR: &str = "***";
	for arg in command.get_args() {
		let mut arg = arg.to_string_lossy().to_string();
		if let Some(access_token) = &access_token {
			arg = arg.replace(&access_token.0, CENSOR_STR);
		}
		o.display(
			MessageContents::ListItem(Box::new(MessageContents::Simple(arg))),
			MessageLevel::Debug,
		);
	}

	o.display(
		MessageContents::Header("Launch command environment".into()),
		MessageLevel::Debug,
	);

	for (env, val) in command.get_envs() {
		let Some(val) = val else { continue };
		let env = env.to_string_lossy().to_string();
		let val = val.to_string_lossy().to_string();

		o.display(
			MessageContents::ListItem(Box::new(MessageContents::Property(
				env,
				Box::new(MessageContents::Simple(val)),
			))),
			MessageLevel::Debug,
		);
	}

	if let Some(dir) = command.get_current_dir() {
		o.display(
			MessageContents::Property(
				"Launch command directory".into(),
				Box::new(MessageContents::Simple(dir.to_string_lossy().into())),
			),
			MessageLevel::Debug,
		);
	}

	Ok(())
}

/// Creates a command wrapped in multiple other wrappers
fn create_wrapped_command(command: &OsStr, wrappers: &[WrapperCommand]) -> Command {
	let mut cmd = Command::new(command);
	for wrapper in wrappers {
		cmd = wrap_single(cmd, wrapper);
	}
	cmd
}

/// Wraps a single command in a wrapper
fn wrap_single(command: Command, wrapper: &WrapperCommand) -> Command {
	let mut new_cmd = Command::new(&wrapper.cmd);
	new_cmd.args(&wrapper.args);
	new_cmd.arg(command.get_program());
	new_cmd.args(command.get_args());
	new_cmd
}

/// Gets the path to an instance stdout / stdin file
pub fn get_stdio_file_path(paths: &Paths, is_stdin: bool) -> PathBuf {
	// We just use the timestamp to keep it unique
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_secs_f64();

	let mode = if is_stdin { "stdin" } else { "stdout" };
	let filename = format!("{mode}_{now}");

	paths.stdio.join(filename)
}

/// Container struct for parameters for launching the game process
pub(crate) struct LaunchGameProcessParameters<'a> {
	/// The base command to run, usually the path to the JVM
	pub command: &'a OsStr,
	/// The current working directory, usually the instance dir
	pub cwd: &'a Path,
	/// The Java main class to run
	pub main_class: Option<&'a str>,
	pub paths: &'a Paths,
	pub props: LaunchProcessProperties,
	pub launch_config: &'a LaunchConfiguration,
	pub version: &'a VersionName,
	pub version_list: &'a [String],
	pub side: &'a InstanceKind,
	pub user_access_token: Option<&'a AccessToken>,
	pub censor_secrets: bool,
	pub pipe_stdin: bool,
}

/// Container struct for parameters for launching a generic Java process
pub struct LaunchProcessParameters<'a> {
	/// The base command to run, usually the path to the JVM
	pub command: &'a OsStr,
	/// The current working directory, usually the instance dir
	pub cwd: &'a Path,
	/// The Java main class to run
	pub main_class: Option<&'a str>,
	/// Properties for launching
	pub props: LaunchProcessProperties,
	/// The launch configuration
	pub launch_config: &'a LaunchConfiguration,
}

/// Properties for launching the game process that are created by
/// the side-specific launch routine
#[derive(Default)]
pub struct LaunchProcessProperties {
	/// Arguments for the JVM
	pub jvm_args: Vec<String>,
	/// Arguments for the game
	pub game_args: Vec<String>,
	/// Additional environment variables to add to the launch command
	pub additional_env_vars: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_wrappers() {
		let wrappers = vec![
			WrapperCommand {
				cmd: "hello".into(),
				args: Vec::new(),
			},
			WrapperCommand {
				cmd: "world".into(),
				args: vec!["foo".into(), "bar".into()],
			},
		];
		let cmd = create_wrapped_command(OsStr::new("run"), &wrappers);
		dbg!(&cmd);
		assert_eq!(cmd.get_program(), OsStr::new("world"));
		let mut args = cmd.get_args();
		assert_eq!(args.next(), Some(OsStr::new("foo")));
		assert_eq!(args.next(), Some(OsStr::new("bar")));
		assert_eq!(args.next(), Some(OsStr::new("hello")));
		assert_eq!(args.next(), Some(OsStr::new("run")));
	}
}
