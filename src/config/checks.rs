use std::{collections::HashMap, fs::File};

use anyhow::Context;
use nitro_config::{instance::InstanceConfig, template::TemplateConfig};
use nitro_pkg::PkgRequest;
use nitro_shared::{
	id::{InstanceID, TemplateID},
	output::{MessageContents, NitroOutput},
	translate,
};
use version_compare::Version;

use crate::io::paths::Paths;

/// Checks and updates the currently installed Nitro version and warns the user
pub fn check_nitro_version(paths: &Paths, o: &mut impl NitroOutput) -> anyhow::Result<()> {
	let path = paths.internal.join("nitro_version");

	if path.exists() {
		let contents = std::fs::read_to_string(&path)?;
		let contents = contents.trim_end();

		let current_version = Version::from(contents).context("Current version failed to parse")?;
		let new_version = Version::from(crate::VERSION).context("New version failed to parse")?;

		if current_version.compare_to(new_version, version_compare::Cmp::Gt) {
			o.display(MessageContents::Warning(translate!(
				o,
				WrongNitroVersion,
				"current" = &contents,
				"new" = crate::VERSION
			)));
		} else {
			std::fs::write(path, crate::VERSION)?;
		}
	} else {
		std::fs::write(path, crate::VERSION)?;
	}

	Ok(())
}

/// Checks packages configured on instances and templates
pub fn check_configured_packages(
	instances: &HashMap<InstanceID, InstanceConfig>,
	templates: &HashMap<TemplateID, TemplateConfig>,
	o: &mut impl NitroOutput,
) {
	for inst in instances.values() {
		if inst.packages.iter().any(|x| {
			PkgRequest::parse(x.get_pkg_id(), nitro_pkg::PkgRequestSource::UserRequire)
				.repository
				.is_none()
		}) {
			o.display(MessageContents::Warning(
				"An instance uses deprecated generic packages".into(),
			));
			return;
		}
	}

	for temp in templates.values() {
		if temp.instance.packages.iter().any(|x| {
			PkgRequest::parse(x.get_pkg_id(), nitro_pkg::PkgRequestSource::UserRequire)
				.repository
				.is_none()
		}) {
			o.display(MessageContents::Warning(
				"A template uses deprecated generic packages".into(),
			));
			return;
		}
	}
}

/// Checks whether this is the first time the launcher has been run.
/// If it is, saves that info so that it will return false the next time
pub fn is_first_run(paths: &Paths) -> bool {
	let path = paths.internal.join("is_first_run");
	let out = !path.exists();
	if out {
		let _ = File::create(path);
	}

	out
}

#[cfg(test)]
mod tests {
	use nitro_config::package::PackageConfigDeser;
	use nitro_shared::output::TestOutput;

	use super::*;

	#[test]
	fn test_package_checking() {
		let mut instances = HashMap::new();
		instances.insert(
			InstanceID::from("foo"),
			InstanceConfig {
				packages: vec![PackageConfigDeser::Basic("bar".into())],
				..Default::default()
			},
		);

		let mut o = TestOutput(Vec::new());
		check_configured_packages(&instances, &HashMap::new(), &mut o);

		assert_eq!(o.0.len(), 1);
	}
}
