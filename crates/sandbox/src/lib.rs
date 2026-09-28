#![warn(missing_docs)]

//! This crate supports a system for sandboxing Minecraft instances
//!
//! # Features:
//!
//! - `schema`: Enable generation of JSON schemas using the `schemars` crate

use nitro_shared::Side;

use crate::group::{CLIENT_POLICY_GROUPS, DEFAULT_POLICY_GROUPS, GroupResolveParams};

/// Policy groups that define multiple rules for the sandboxing
pub mod group;
/// Linux landlock sandboxing
#[cfg(target_os = "linux")]
pub mod linux;
/// Definitions of rules for the sandboxing
pub mod policy;

/// Uses information about the instance to resolve policy groups
pub fn resolve(
	policy: &policy::SandboxPolicy,
	params: GroupResolveParams<'_>,
) -> anyhow::Result<policy::ResolvedSandboxPolicy> {
	let mut resolved = policy::ResolvedSandboxPolicy::default();

	let mut groups = DEFAULT_POLICY_GROUPS.to_vec();
	if params.side == Side::Client {
		groups.extend(CLIENT_POLICY_GROUPS.iter().copied());
	}
	groups.extend(policy.allowed.iter().copied());
	groups.retain(|x| !policy.disallowed.contains(x));

	for group in groups {
		group.resolve(&params, &mut resolved);
	}

	resolved.allowed_paths.extend(policy.allowed_paths.clone());
	resolved.allowed_hosts.extend(policy.allowed_hosts.clone());

	Ok(resolved)
}

/// Applies the sandboxing policy to the current thread
#[allow(unused_variables)]
pub fn apply(policy: policy::ResolvedSandboxPolicy) -> anyhow::Result<()> {
	#[cfg(target_os = "linux")]
	{
		crate::linux::apply(policy)
	}
	#[cfg(not(target_os = "linux"))]
	{
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::*;

	fn resolve_params() -> GroupResolveParams<'static> {
		GroupResolveParams {
			side: Side::Client,
			instance_dir: Path::new("/instance"),
			java_installation: Path::new("/"),
			jars_dir: Path::new("/"),
			assets_dir: Path::new("/"),
			libraries_dir: Path::new("/"),
			natives_dir: Path::new("/"),
			versions_dir: Path::new("/"),
			stdin_file: None,
			stdout_file: None,
		}
	}

	#[test]
	fn test_resolve_default() {
		let policy = policy::SandboxPolicy::default();
		let resolved = resolve(&policy, resolve_params()).unwrap();
		assert!(
			resolved
				.allowed_paths
				.contains_key(&String::from("/instance"))
		);
	}

	#[test]
	fn test_resolve_disallow_all_defaults() {
		let mut disallowed = DEFAULT_POLICY_GROUPS.to_vec();
		disallowed.extend(CLIENT_POLICY_GROUPS.iter().copied());
		let policy = policy::SandboxPolicy {
			allowed: vec![],
			disallowed,
			..Default::default()
		};
		let resolved = resolve(&policy, resolve_params()).unwrap();
		dbg!(&resolved);
		assert!(resolved.allowed_paths.is_empty());
	}
}
