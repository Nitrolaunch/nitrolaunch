use std::path::Path;

use itertools::Itertools;
use landlock::{
	ABI, Access, AccessFs, AccessNet, NetPort, PathBeneath, PathFd, PathFdError, Ruleset,
	RulesetAttr, RulesetCreated, RulesetCreatedAttr, RulesetError,
};

use crate::policy::{FilesystemPolicy, ResolvedSandboxPolicy};

const LL_VERSION: ABI = ABI::V5;

#[derive(Debug, thiserror::Error)]
enum RestrictionError {
	#[error(transparent)]
	Ruleset(#[from] RulesetError),
	#[error(transparent)]
	AddRule(#[from] PathFdError),
}

/// Applies the sandboxing policy to the current thread
pub fn apply(policy: ResolvedSandboxPolicy) -> anyhow::Result<()> {
	let ruleset = create_ruleset(&policy)?;

	ruleset.restrict_self()?;

	Ok(())
}

fn create_ruleset(policy: &ResolvedSandboxPolicy) -> Result<RulesetCreated, RestrictionError> {
	let mut ruleset = create_base_ruleset()?;

	for (path, fs_policy) in policy.allowed_paths.iter().sorted_by_key(|x| x.0.clone()) {
		if !Path::new(path).exists() {
			continue;
		}

		let access = match fs_policy {
			FilesystemPolicy::Read => AccessFs::from_read(LL_VERSION),
			FilesystemPolicy::ReadWrite => AccessFs::from_all(LL_VERSION),
			FilesystemPolicy::Execute => AccessFs::Execute.into(),
		};

		ruleset = ruleset.add_rule(PathBeneath::new(PathFd::new(path)?, access))?;
	}

	for port in policy.allowed_ports.iter().sorted() {
		ruleset = ruleset.add_rule(NetPort::new(*port, AccessNet::from_all(LL_VERSION)))?;
	}

	Ok(ruleset)
}

fn create_base_ruleset() -> Result<RulesetCreated, RestrictionError> {
	let disallow_all_fs = AccessFs::from_all(LL_VERSION) | AccessFs::Execute;
	let disallow_all_net = AccessNet::from_all(LL_VERSION);

	Ok(Ruleset::default()
		.handle_access(disallow_all_fs)?
		.handle_access(disallow_all_net)?
		.create()?)
}
