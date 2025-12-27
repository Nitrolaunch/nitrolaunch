use std::{
	hash::{DefaultHasher, Hash, Hasher},
	path::PathBuf,
};

use anyhow::Context;
use nitro_core::io::{json_from_file, json_to_file_pretty};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

use crate::io::paths::Paths;

/// A registry of running instances
pub struct RunningInstanceRegistry {
	data: RunningInstanceRegistryDeser,
	/// Whether the contents have changed and we need to write
	is_dirty: bool,
	system: System,
	path: PathBuf,
}

impl RunningInstanceRegistry {
	fn get_path(paths: &Paths) -> PathBuf {
		paths.internal.join("running_instances.json")
	}

	/// Open the registry. This will hold the registry file descriptor until dropped.
	pub fn open(paths: &Paths) -> anyhow::Result<Self> {
		let path = Self::get_path(paths);
		let data = if path.exists() {
			json_from_file(&path).context("Failed to open registry file")?
		} else {
			RunningInstanceRegistryDeser::default()
		};

		let system = System::new_all();

		let mut out = Self {
			data,
			is_dirty: false,
			system,
			path,
		};

		// Remove any dead instances so we start with a good state
		out.remove_dead_instances();

		Ok(out)
	}

	/// Re-reads the registry
	pub fn reread(&mut self) -> anyhow::Result<()> {
		let data = json_from_file(&self.path).context("Failed to read registry file")?;
		self.data = data;

		Ok(())
	}

	/// Writes data from the in-memory registry to the file
	pub fn write(&mut self) -> anyhow::Result<()> {
		if !self.is_dirty {
			return Ok(());
		}

		json_to_file_pretty(&self.path, &self.data).context("Failed to write to registry file")?;

		self.is_dirty = false;

		Ok(())
	}

	/// Gets whether the registry is dirty (has changes that need to be written)
	pub fn is_dirty(&self) -> bool {
		self.is_dirty
	}

	/// Gets a hash of the current list of instances
	pub fn get_entries_hash(&mut self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.data.instances.hash(&mut hasher);
		hasher.finish()
	}

	/// Removes instances that aren't alive from the registry
	pub fn remove_dead_instances(&mut self) {
		self.system
			.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

		let original_lenth = self.data.instances.len();
		self.data
			.instances
			.retain(|x| is_process_alive(x.pid, &self.system, x.is_java));

		if original_lenth != self.data.instances.len() {
			self.is_dirty = true;
		}
	}

	/// Adds an instance to the registry
	pub fn add_instance(&mut self, pid: u32, instance: &str, is_java: bool) {
		let entry = RunningInstanceEntry {
			pid,
			parent_pid: std::process::id(),
			instance_id: instance.to_string(),
			is_java,
		};
		self.data.instances.push(entry);
		self.is_dirty = true;
	}

	/// Removes an instance from the registry
	pub fn remove_instance(&mut self, pid: u32, instance: &str) {
		let index = self
			.data
			.instances
			.iter()
			.position(|x| x.pid == pid && x.instance_id == instance);

		if let Some(index) = index {
			self.data.instances.remove(index);
		}

		self.is_dirty = true;
	}

	/// Kills an instance in the registry
	pub fn kill_instance(&mut self, instance: &str) {
		let mut pids = Vec::new();
		for entry in &self.data.instances {
			if entry.instance_id == instance {
				pids.push(entry.pid);
			}
		}

		for pid in pids {
			let pid2 = Pid::from_u32(pid);
			let process = self.system.process(pid2);
			if let Some(process) = process {
				process.kill();
			}

			self.remove_instance(pid, instance);
		}
	}

	/// Tries to check if an instance is alive
	pub fn is_instance_alive(&self, entry: &RunningInstanceEntry) -> bool {
		is_process_alive(entry.pid, &self.system, entry.is_java)
	}

	/// Iterates over the entries in the registry
	pub fn iter_entries(&self) -> impl Iterator<Item = &RunningInstanceEntry> {
		self.data.instances.iter()
	}
}

impl Drop for RunningInstanceRegistry {
	fn drop(&mut self) {
		let _ = self.write();
	}
}

#[derive(Deserialize, Serialize, Default, Debug)]
struct RunningInstanceRegistryDeser {
	instances: Vec<RunningInstanceEntry>,
}

/// An entry for a running instance in the registry
#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct RunningInstanceEntry {
	/// The ID of the instance process
	pub pid: u32,
	/// The ID of this instance
	pub instance_id: String,
	/// The PID of the process that launched this instance
	pub parent_pid: u32,
	/// Whether this is a Java instance
	#[serde(default = "default_is_java")]
	pub is_java: bool,
}

fn default_is_java() -> bool {
	true
}

/// Checks if an instance process is alive
pub fn is_process_alive(pid: u32, system: &System, is_java: bool) -> bool {
	let pid = Pid::from_u32(pid);

	let process = system.process(pid);
	// The process doesn't exist
	let Some(process) = process else {
		return false;
	};

	// If there is no Java, and it should be, it probably isn't our process
	if is_java
		&& !process.name().to_string_lossy().contains("java")
		&& !process
			.cmd()
			.iter()
			.any(|x| x.to_string_lossy().contains("java"))
	{
		return false;
	}

	true
}
