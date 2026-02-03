use nitro_core::io::logs::{list_logs, read_log};
use nitro_plugin::hook::hooks::{
	GetInstanceLog, GetInstanceLogArg, GetInstanceLogs, GetInstanceLogsArg,
};
use nitro_shared::output::NitroOutput;

use crate::{instance::Instance, io::paths::Paths, plugin::PluginManager};

impl Instance {
	/// Get the list of log IDs for this instance
	pub async fn get_logs(
		&mut self,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<Vec<String>> {
		self.ensure_dirs(paths)?;

		if let Some(plugin) = &self
			.config
			.original_config_with_templates_and_plugins
			.custom_logging_plugin
		{
			let arg = GetInstanceLogsArg {
				id: self.id.to_string(),
				config: self
					.config
					.original_config_with_templates_and_plugins
					.clone(),
			};

			let result = plugins
				.call_hook_on_plugin(GetInstanceLogs, plugin, &arg, paths, o)
				.await?;
			let Some(result) = result else {
				return Ok(Vec::new());
			};

			result.result(o).await
		} else {
			if let Some(game_dir) = &self.dirs.get().game_dir {
				let logs_dir = game_dir.join("logs");
				list_logs(&logs_dir)
			} else {
				Ok(Vec::new())
			}
		}
	}

	/// Get the contents of a specific log file
	pub async fn get_log(
		&mut self,
		log_id: &str,
		plugins: &PluginManager,
		paths: &Paths,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<String> {
		self.ensure_dirs(paths)?;

		if let Some(plugin) = &self
			.config
			.original_config_with_templates_and_plugins
			.custom_logging_plugin
		{
			let arg = GetInstanceLogArg {
				instance_id: self.id.to_string(),
				log_id: log_id.to_string(),
				config: self
					.config
					.original_config_with_templates_and_plugins
					.clone(),
			};

			let result = plugins
				.call_hook_on_plugin(GetInstanceLog, plugin, &arg, paths, o)
				.await?;
			let Some(result) = result else {
				return Ok(String::new());
			};

			result.result(o).await
		} else {
			if let Some(game_dir) = &self.dirs.get().game_dir {
				let logs_dir = game_dir.join("logs");
				read_log(&logs_dir.join(log_id))
			} else {
				Ok(String::new())
			}
		}
	}
}
