mod config;
mod files;
mod instance;
mod package;
mod plugin;
mod template;
mod user;

use std::collections::HashMap;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use color_print::{cformat, cprintln};

use nitrolaunch::config::modifications::{apply_modifications_and_write, ConfigModification};
use nitrolaunch::config::Config;
use nitrolaunch::config_crate::ConfigDeser;
use nitrolaunch::instance::transfer::{load_formats, migrate_instances};
use nitrolaunch::io::lock::Lockfile;
use nitrolaunch::io::paths::Paths;
use nitrolaunch::plugin::PluginManager;
use nitrolaunch::plugin_crate::hook::hooks::{self, AddTranslations, SubcommandArg};
use nitrolaunch::plugin_crate::plugin::PluginProvidedSubcommand;
use nitrolaunch::shared::id::InstanceID;
use nitrolaunch::shared::lang::translate::TranslationKey;
use nitrolaunch::shared::later::Later;
use nitrolaunch::shared::output::{MessageContents, MessageLevel, NitroOutput};

use self::config::ConfigSubcommand;
use self::files::FilesSubcommand;
use self::instance::InstanceSubcommand;
use self::package::PackageSubcommand;
use self::plugin::PluginSubcommand;
use self::template::TemplateSubcommand;
use self::user::UserSubcommand;

use super::output::TerminalOutput;

#[derive(Debug, Subcommand)]
pub enum Command {
	#[command(about = "Manage instances")]
	#[clap(alias = "inst")]
	Instance {
		#[command(subcommand)]
		command: InstanceSubcommand,
	},
	#[command(about = "Manage users and authentication")]
	User {
		#[command(subcommand)]
		command: UserSubcommand,
	},
	#[command(about = "Launch instances to play the game")]
	Launch {
		/// The instance to launch
		instance: Option<String>,
	},
	#[command(about = "Manage packages")]
	#[clap(alias = "pkg")]
	Package {
		#[command(subcommand)]
		command: PackageSubcommand,
	},
	#[command(about = "Manage plugins")]
	#[clap(alias = "plug")]
	Plugin {
		#[command(subcommand)]
		command: PluginSubcommand,
	},
	#[command(about = "Manage configuration")]
	#[clap(alias = "cfg", alias = "conf")]
	Config {
		#[command(subcommand)]
		command: ConfigSubcommand,
	},
	#[command(about = "Print the Nitrolaunch version")]
	Version,
	#[command(about = "Deal with files created by Nitrolaunch")]
	Files {
		#[command(subcommand)]
		command: FilesSubcommand,
	},
	#[command(about = "Do operations with instance templates")]
	Template {
		#[command(subcommand)]
		command: TemplateSubcommand,
	},
	#[command(about = "Import instances from another launcher")]
	Migrate {
		/// Which format to use
		format: Option<String>,
		/// Specific instances to migrate. Will migrate all if none are specified
		#[arg(short = 'i', long)]
		instances: Vec<String>,
		/// Whether to copy the instance files. By default, will link to the existing ones instead.
		#[arg(short = 'c', long)]
		copy: bool,
	},
	#[clap(external_subcommand)]
	External(Vec<String>),
}

#[derive(Debug, Parser)]
pub struct Cli {
	#[command(subcommand)]
	command: Command,
	#[arg(short, long)]
	debug: bool,
	#[arg(short = 'D', long)]
	trace: bool,
}

/// Run the command line interface
pub async fn run_cli() -> anyhow::Result<()> {
	// Parse the CLI
	let cli = Cli::try_parse();
	if let Err(e) = &cli {
		if let clap::error::ErrorKind::DisplayHelp
		| clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
		| clap::error::ErrorKind::DisplayVersion = e.kind()
		{
			println!("{e}");
			return Ok(());
		} else {
			eprintln!("{}", cformat!("<r>{e}"));
			bail!("");
		}
	}
	let cli = cli?;

	// Prepare the command data
	let paths = Paths::new()
		.await
		.context("Failed to set up system paths")?;
	let mut output = TerminalOutput::new(&paths).context("Failed to set up output")?;
	let res = {
		let mut data = CmdData::new(paths, &mut output)?;
		let log_level = get_log_level(&cli);
		data.output.set_log_level(log_level);

		match cli.command {
			Command::User { command } => user::run(command, &mut data).await,
			Command::Launch { instance } => {
				instance::launch(instance, None, false, false, data).await
			}
			Command::Version => {
				print_version();
				Ok(())
			}
			Command::Files { command } => files::run(command, &mut data).await,
			Command::Package { command } => package::run(command, &mut data).await,
			Command::Instance { command } => instance::run(command, data).await,
			Command::Plugin { command } => plugin::run(command, &mut data).await,
			Command::Config { command } => config::run(command, &mut data).await,
			Command::Template { command } => template::run(command, &mut data).await,
			Command::Migrate {
				format,
				instances,
				copy,
			} => migrate(format, instances, copy, &mut data).await,
			Command::External(args) => call_plugin_subcommand(args, None, &mut data).await,
		}
	};

	if let Err(e) = &res {
		// Don't use the existing process or section
		output.end_process();
		output.end_section();
		output.display(
			MessageContents::Error(format!("{e:?}")),
			MessageLevel::Important,
		);
	}

	res
}

/// Get the log level based on the debug options
fn get_log_level(cli: &Cli) -> MessageLevel {
	if cli.trace {
		MessageLevel::Trace
	} else if cli.debug {
		MessageLevel::Debug
	} else {
		MessageLevel::Important
	}
}

/// Data passed to commands
pub struct CmdData<'a> {
	pub paths: Paths,
	pub config: Later<Config>,
	pub output: &'a mut TerminalOutput,
}

impl<'a> CmdData<'a> {
	pub fn new(paths: Paths, output: &'a mut TerminalOutput) -> anyhow::Result<Self> {
		Ok(Self {
			paths,
			config: Later::new(),
			output,
		})
	}

	/// Ensure that the config is loaded
	pub async fn ensure_config(&mut self, show_warnings: bool) -> anyhow::Result<()> {
		if self.config.is_empty() {
			let plugins = PluginManager::load(&self.paths, self.output)
				.await
				.context("Failed to load plugins configuration")?;

			self.config.fill(
				Config::load(
					&Config::get_path(&self.paths),
					plugins,
					show_warnings,
					&self.paths,
					crate::secrets::get_ms_client_id(),
					self.output,
				)
				.await
				.context("Failed to load config")?,
			);
		}

		// Update the translation map from plugins
		let mut results = self
			.config
			.get()
			.plugins
			.call_hook(AddTranslations, &(), &self.paths, self.output)
			.await
			.context("Failed to get extra translations from plugins")?;

		while let Some(mut result) = results.next_result(self.output).await? {
			let map = result.remove(&self.config.get().prefs.language);
			if let Some(map) = map {
				self.output.set_translation_map(map);
			}
		}

		Ok(())
	}

	/// Get the raw deserialized config
	pub fn get_raw_config(&self) -> anyhow::Result<ConfigDeser> {
		let config =
			Config::open(&Config::get_path(&self.paths)).context("Failed to open config")?;

		Ok(config)
	}
}

/// Print the Nitrolaunch version
fn print_version() {
	let version = env!("CARGO_PKG_VERSION");
	let nitrolaunch_version = nitrolaunch::VERSION;
	cprintln!("CLI version: <g>{}</g>", version);
	cprintln!("Nitrolaunch version: <g>{}</g>", nitrolaunch_version);
}

/// Runs instance migration
async fn migrate(
	format: Option<String>,
	instances: Vec<String>,
	copy: bool,
	data: &mut CmdData<'_>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	// Figure out the format
	let formats = load_formats(&config.plugins, &data.paths, data.output)
		.await
		.context("Failed to get available transfer formats")?;

	let format = if let Some(format) = &format {
		format
	} else {
		let options: Vec<_> = formats.iter_format_names().collect();
		if options.is_empty() {
			bail!(
				"{}",
				data.output.translate(TranslationKey::NoTransferFormats)
			);
		}
		inquire::Select::new("What launcher do you want to import from?", options).prompt()?
	};

	let mut lock = Lockfile::open(&data.paths).context("Failed to open lockfile")?;

	let new_configs = migrate_instances(
		format,
		Some(instances).filter(|x| !x.is_empty()),
		!copy,
		&formats,
		&config.plugins,
		&data.paths,
		&mut lock,
		data.output,
	)
	.await
	.context("Failed to migrate instances")?;

	lock.finish(&data.paths)?;

	let mut config2 = data.get_raw_config()?;

	for key in new_configs.keys() {
		if config2
			.instances
			.contains_key(&InstanceID::from(key.clone()))
		{
			bail!("Duplicate instance ID {key}");
		}
	}

	apply_modifications_and_write(
		&mut config2,
		new_configs
			.into_iter()
			.map(|(id, config)| ConfigModification::AddInstance(id.into(), config))
			.collect(),
		&data.paths,
		&config.plugins,
		data.output,
	)
	.await
	.context("Failed to write modified config")?;

	Ok(())
}

/// Call a plugin subcommand
async fn call_plugin_subcommand(
	args: Vec<String>,
	supercommand: Option<&str>,
	data: &mut CmdData<'_>,
) -> anyhow::Result<()> {
	data.ensure_config(true).await?;
	let config = data.config.get();

	// Make sure the subcommand is handled by one of the plugins
	let subcommand = args
		.first()
		.context("Subcommand does not have first argument")?;

	{
		let lock = config.plugins.get_lock().await;
		let exists = lock.manager.iter_plugins().any(|x| {
			x.get_manifest()
				.subcommands
				.iter()
				.any(|x| {
					if x.0 != subcommand {
						return false;
					}

					if let Some(supercommand2) = supercommand {
						matches!(x.1, PluginProvidedSubcommand::Specific { supercommand, .. } if supercommand == supercommand2)
					} else {
						matches!(x.1, PluginProvidedSubcommand::Global(..))
					}
				})
		});
		if !exists {
			bail!("Subcommand '{subcommand}' does not exist");
		}
	}

	let mut instance_configs = HashMap::new();
	for (id, instance) in &config.instances {
		instance_configs.insert(
			id.clone(),
			instance
				.get_config()
				.original_config_with_templates_and_plugins
				.clone(),
		);
	}

	let arg = SubcommandArg {
		args,
		instances: instance_configs,
	};

	let results = config
		.plugins
		.call_hook(hooks::Subcommand, &arg, &data.paths, data.output)
		.await
		.context("Plugin subcommand failed")?;
	results.all_results(data.output).await?;

	Ok(())
}
