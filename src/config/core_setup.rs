use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use nitro_core::{
	auth_crate::mc::ClientId,
	config::BrandingProperties,
	io::java::install::{CustomJavaFunction, CustomJavaFunctionResult},
	NitroCore,
};
use nitro_plugin::hook::hooks::{AddVersions, InstallCustomJava, InstallCustomJavaArg};
use nitro_shared::{
	output::{NitroOutput, NoOp},
	UpdateDepth,
};
use reqwest::Client;

use crate::{instance::update::manager::UpdateSettings, io::paths::Paths, plugin::PluginManager};

/// Sets up and configures a NitroCore according to Nitrolaunch's features
pub async fn setup_core(
	client_id: Option<&ClientId>,
	settings: &UpdateSettings,
	client: &Client,
	plugins: &PluginManager,
	paths: &Paths,
	o: &mut impl NitroOutput,
) -> anyhow::Result<NitroCore> {
	let mut core_config = nitro_core::ConfigBuilder::new().branding(BrandingProperties::new(
		"Nitrolaunch".into(),
		crate::VERSION.into(),
	));
	if let Some(client_id) = client_id {
		core_config = core_config.ms_client_id(client_id.clone());
	}
	let core_config = core_config.build();
	let mut core = NitroCore::with_config(core_config).context("Failed to initialize core")?;

	// Set up custom plugin integrations
	core.set_custom_java_install_fn(Arc::new(JavaFunction {
		plugins: plugins.clone(),
		paths: paths.clone(),
	}));

	core.set_client(client.clone());

	// Add extra versions to manifest from plugins
	let mut results = plugins
		.call_hook(AddVersions, &settings.depth, paths, o)
		.await
		.context("Failed to call add_versions hook")?;
	while let Some(result) = results.next_result(o).await? {
		core.add_additional_versions(result);
	}

	Ok(core)
}

/// CustomJavaFunction implementation using plugins
struct JavaFunction {
	plugins: PluginManager,
	paths: Paths,
}

#[async_trait::async_trait]
impl CustomJavaFunction for JavaFunction {
	async fn install(
		&self,
		java: &str,
		major_version: &str,
		update_depth: UpdateDepth,
	) -> anyhow::Result<Option<CustomJavaFunctionResult>> {
		let arg = InstallCustomJavaArg {
			kind: java.to_string(),
			major_version: major_version.to_string(),
			update_depth,
		};
		let mut results = self
			.plugins
			.call_hook(InstallCustomJava, &arg, &self.paths, &mut NoOp)
			.await
			.context("Failed to call install custom Java hook")?;

		let mut out = None;
		while let Some(result) = results.next_result(&mut NoOp).await? {
			if result.is_some() {
				out = result;
			}
		}

		if let Some(out) = out {
			Ok(Some(CustomJavaFunctionResult {
				path: PathBuf::from(out.path),
				version: out.version,
			}))
		} else {
			Ok(None)
		}
	}
}
