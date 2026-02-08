use std::time::Duration;

use discord_rich_presence::{
	activity::{Activity, ActivityType, StatusDisplayType, Timestamps},
	DiscordIpc, DiscordIpcClient,
};
use nitro_plugin::api::executable::ExecutablePlugin;
use nitro_shared::{
	output::{MessageContents, NitroOutput},
	util::utc_timestamp,
};
use nitrolaunch::{instance::tracking::RunningInstanceRegistry, io::paths::Paths};

/// Discord app client ID
static CLIENT_ID: &str = "1399096265366573249";

fn main() -> anyhow::Result<()> {
	let mut plugin =
		ExecutablePlugin::from_manifest_file("discord_status", include_str!("plugin.json"))?;

	plugin.while_instance_launch(|mut ctx, _| loop {
		std::thread::sleep(Duration::from_secs(5));

		if let Err(e) = update_presence() {
			ctx.get_output().debug(MessageContents::Error(format!(
				"Failed to update Discord rich presence:\n{e}"
			)));
		}
	})?;

	plugin.on_instance_stop(|mut ctx, _| {
		if let Err(e) = update_presence() {
			ctx.get_output().debug(MessageContents::Error(format!(
				"Failed to update Discord rich presence:\n{e}"
			)));
		}

		ctx.get_output().debug(MessageContents::Success(
			"Discord rich presence updated".into(),
		));

		Ok(())
	})?;

	Ok(())
}

fn update_presence() -> anyhow::Result<()> {
	let mut registry = RunningInstanceRegistry::open(&Paths::new_no_create()?)?;
	registry.remove_dead_instances();

	let mut client = DiscordIpcClient::new(CLIENT_ID);
	client.connect()?;

	let instance_count = registry.iter_entries().count();
	if instance_count > 0 {
		let payload = Activity::new()
			.status_display_type(StatusDisplayType::Name)
			.state(format!("{instance_count} instances running"))
			.activity_type(ActivityType::Playing)
			.timestamps(Timestamps::new().start(utc_timestamp().unwrap_or_default() as i64));
		client.set_activity(payload)?;
	} else {
		client.clear_activity()?;
	}

	Ok(())
}
