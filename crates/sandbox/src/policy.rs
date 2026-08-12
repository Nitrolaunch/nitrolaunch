use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::group::PolicyGroup;

/// Configuration for the sandboxing system
#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct SandboxPolicy {
	/// Additional allowed policy groups
	pub allowed: Vec<PolicyGroup>,
	/// List of filesystem paths and their access policies for the sandboxed instance
	pub allowed_paths: HashMap<String, FilesystemPolicy>,
	/// List of IP addresses or hostnames that the sandboxed Minecraft instance can connect to
	pub allowed_hosts: Vec<String>,
}

/// Defines access to a filesystem path for the sandboxed instance
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemPolicy {
	/// File or directory can only be read
	Read,
	/// File or directory can be read and written to
	#[serde(alias = "write")]
	ReadWrite,
    /// File can be executed
    Execute,
}

/// Resolved version of the sandbox policy, with all groups expanded into their individual rules
#[derive(Debug)]
pub struct ResolvedSandboxPolicy {
	pub(crate) allowed_paths: HashMap<String, FilesystemPolicy>,
	pub(crate) allowed_hosts: Vec<String>,
}
