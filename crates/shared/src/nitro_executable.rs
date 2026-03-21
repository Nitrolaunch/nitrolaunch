use std::{
	collections::HashSet,
	fs::{File, Permissions},
	path::{Path, PathBuf},
	process::Command,
};
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

/// Stored registry of executable
pub struct NitroExecutableRegistry {
	executables: HashSet<NitroExecutable>,
	path: PathBuf,
}

impl NitroExecutableRegistry {
	/// Opens the registry in the given data/internal dir
	pub fn open(internal_dir: &Path) -> anyhow::Result<Self> {
		let path = internal_dir.join("nitro_executables.json");
		let data = if path.exists() {
			serde_json::from_reader(File::open(&path)?)?
		} else {
			HashSet::new()
		};

		Ok(Self {
			executables: data,
			path,
		})
	}

	fn write(&self) -> anyhow::Result<()> {
		if let Some(parent) = self.path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		self.write_shell_script()
			.context("Failed to write shell script")?;
		serde_json::to_writer(File::create(&self.path)?, &self.executables)
			.context("Failed to write JSON")
	}

	/// Writes the simple shell script for other applications to use
	fn write_shell_script(&self) -> anyhow::Result<()> {
		let Some(best_executable) = self.get_best_executable() else {
			return Ok(());
		};
		let script = best_executable.to_shell_script();

		#[cfg(any(target_os = "windows", target_family = "unix"))]
		{
			#[cfg(target_os = "windows")]
			let filename = "launch_instance.bat";
			#[cfg(target_family = "unix")]
			let filename = "launch_instance.sh";
	
			let Some(parent) = self.path.parent() else {
				return Ok(());
			};
	
			let path = parent.join(filename);
			std::fs::write(&path, script).context("Failed to write script")?;
			#[cfg(target_family = "unix")]
			std::fs::set_permissions(path, Permissions::from_mode(0o775))?;
		}

		Ok(())
	}

	/// Adds the current executable to the registry and writes to disk
	pub fn add_this(&mut self, client_id: NitroClientId) -> anyhow::Result<()> {
		let path = std::env::current_exe()?;
		if !path.exists() {
			bail!("Executable file does not exist");
		}

		self.executables.insert(NitroExecutable {
			path: path.to_string_lossy().to_string(),
			client_id: client_id,
		});

		self.write()
	}

	/// Launches an instance using the best available executable
	pub fn launch_instance(&self, instance: &str, account: Option<&str>) -> Option<Command> {
		let executable = self.get_best_executable()?;

		Some(executable.launch_instance(instance, account))
	}

	/// Gets the best available executable
	fn get_best_executable(&self) -> Option<&NitroExecutable> {
		if let Some(cli) = self
			.executables
			.iter()
			.find(|x| x.client_id == NitroClientId::Cli && Path::new(&x.path).exists())
		{
			Some(cli)
		} else {
			self.executables
				.iter()
				.find(|x| Path::new(&x.path).exists())
		}
	}
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
struct NitroExecutable {
	path: String,
	client_id: NitroClientId,
}

impl NitroExecutable {
	fn launch_instance(&self, instance: &str, account: Option<&str>) -> Command {
		let mut command = Command::new(&self.path);
		match &self.client_id {
			NitroClientId::Cli => {
				command.arg("instance");
				command.arg("launch");
				command.arg(instance);
				if let Some(account) = account {
					command.arg("--account");
					command.arg(account);
				}
			}
			NitroClientId::Gui | NitroClientId::Other(..) => {
				command.arg("--launch");
				command.arg(instance);
				if let Some(account) = account {
					command.arg("--account");
					command.arg(account);
				}
			}
		}

		command
	}

	fn to_shell_script(&self) -> String {
		match &self.client_id {
			NitroClientId::Cli => {
				#[cfg(target_os = "windows")]
				return format!("{} instance launch %*\n", self.path);
				#[cfg(target_family = "unix")]
				return format!("{} instance launch $@\n", self.path);
				#[cfg(not(any(target_os = "windows", target_family = "unix")))]
				return "echo Not supported on this system".into();
			}
			NitroClientId::Gui | NitroClientId::Other(..) => {
				#[cfg(target_os = "windows")]
				return format!("{} --launch %*\n", self.path);
				#[cfg(target_family = "unix")]
				return format!("{} --launch $@\n", self.path);
				#[cfg(not(any(target_os = "windows", target_family = "unix")))]
				return "echo Not supported on this system".into();
			}
		}
	}
}

/// Type of launcher interface
#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NitroClientId {
	/// CLI
	Cli,
	/// GUI
	Gui,
	/// Unknown
	#[serde(untagged)]
	Other(String),
}
