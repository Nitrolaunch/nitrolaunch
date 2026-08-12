#![warn(missing_docs)]

//! This crate supports a system for sandboxing Minecraft instances
//!
//! # Features:
//!
//! - `schema`: Enable generation of JSON schemas using the `schemars` crate

use nitro_shared::Side;

use crate::group::{DEFAULT_POLICY_GROUPS, GroupResolveParams, PolicyGroup};

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
	groups.extend(policy.allowed.iter().copied());
	if params.side == Side::Client {
		groups.push(PolicyGroup::GraphicsDevices);
		groups.push(PolicyGroup::InputDevices);
	}

	for group in groups {
		group.resolve(&params, &mut resolved);
	}

	resolved.allowed_paths.extend(policy.allowed_paths.clone());
	resolved.allowed_hosts.extend(policy.allowed_hosts.clone());

	Ok(resolved)
}

/// Applies the sandboxing policy to the current thread
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
