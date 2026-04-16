use std::cmp::Reverse;
use std::fs::File;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use chrono::DateTime;
use clap::Parser;
use itertools::Itertools;
use nitro_plugin::api::wasm::output::WASMPluginOutput;
use nitro_plugin::api::wasm::sys::get_data_dir;
use nitro_plugin::api::wasm::util::{
	get_custom_config, get_persistent_state, set_persistent_state,
};
use nitro_plugin::api::wasm::WASMPlugin;
use nitro_plugin::hook::hooks::{InstanceTile, InstanceTileSize};
use nitro_plugin::nitro_wasm_plugin;
use nitro_shared::output::{MessageContents, NitroOutput};
use nitro_shared::util::utc_timestamp;
use serde::{Deserialize, Serialize};

nitro_wasm_plugin!(main, "stats");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.subcommand(|arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "stats" {
			return Ok(());
		}
		// Trick the parser to give it the right bin name
		let it = std::iter::once(format!("nitro {subcommand}")).chain(arg.args.into_iter().skip(1));
		Cli::try_parse_from(it)?;
		print_stats()?;

		Ok(())
	})?;

	plugin.on_instance_launch(|arg| {
		let Ok(mut stats) = Stats::open() else {
			WASMPluginOutput::new().display(MessageContents::Error("Failed to open stats".into()));
			return Ok(());
		};

		// Write launch count and launch time
		let entry = stats.instances.entry(arg.id.clone()).or_default();
		entry.launches += 1;
		if let Ok(timestamp) = utc_timestamp() {
			entry.last_launch = Some(timestamp);
		}
		let _ = stats.write();

		// Track when the instance started in persistent state to get playtime
		let mut state: HashMap<String, u64> = get_persistent_state().unwrap_or_default();
		state.insert(arg.id.clone(), utc_timestamp()?);
		set_persistent_state(&state);

		Ok(())
	})?;

	plugin.on_instance_stop(|arg| update_playtime(&arg.id, false))?;

	plugin.while_instance_launch(|arg| {
		let config = get_custom_config().unwrap_or("{}".into());
		let config: Config =
			serde_json::from_str(&config).context("Failed to deserialize custom config")?;

		if !config.live_tracking {
			return Ok(());
		}

		loop {
			std::thread::sleep(Duration::from_secs(10));

			let res = update_playtime(&arg.id, true).context("Failed to update playtime");
			if let Err(e) = res {
				println!("$_{e:?}");
			}
		}
	})?;

	plugin.add_instance_tiles(|arg| {
		let stats = Stats::open().context("Failed to open stats")?;

		let default = InstanceStats::default();
		let stats = stats.instances.get(&arg).unwrap_or(&default);

		Ok(vec![InstanceTile {
			id: "stats".into(),
			contents: format_stat_card(stats),
			size: InstanceTileSize::Small,
		}])
	})?;

	Ok(())
}

/// Update the playtime. When update_state is true, the persistent state is updated for the next update
/// so that the time delta is correct
fn update_playtime(instance: &str, update_state: bool) -> anyhow::Result<()> {
	let mut state: HashMap<String, u64> = get_persistent_state().unwrap_or_default();
	let Some(start_time) = state.get_mut(instance) else {
		return Ok(());
	};
	let now = utc_timestamp()?;
	let diff_minutes = (now - *start_time) / 60;

	if diff_minutes > 0 {
		let mut stats = Stats::open().context("Failed to open stats")?;
		stats
			.instances
			.entry(instance.to_string())
			.or_default()
			.playtime += diff_minutes;

		// Update start time so that the next update doesn't grow exponentially, but only if we actually made a difference to the number of minutes
		if update_state {
			*start_time = now;
			set_persistent_state(&state);
		}

		stats.write().context("Failed to write stats")?;
	}

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {}

fn print_stats() -> anyhow::Result<()> {
	let stats = Stats::open().context("Failed to open stats")?;

	#[derive(PartialEq, Eq, PartialOrd, Ord)]
	struct Ordering {
		playtime: Reverse<u64>,
		launches: Reverse<u32>,
		instance_id: String,
	}

	let total: u64 = stats
		.instances
		.values()
		.map(|x| x.calculate_playtime())
		.sum();
	let total = format_time(total);
	println!("Total playtime: {total}");

	for (instance, stats) in stats
		.instances
		.into_iter()
		.sorted_by_key(|(inst_id, stats)| Ordering {
			launches: Reverse(stats.launches),
			playtime: Reverse(stats.calculate_playtime()),
			instance_id: inst_id.clone(),
		}) {
		println!(
			" - {instance} - Launched {} times for a total of {}",
			stats.launches,
			format_time(stats.calculate_playtime())
		);
	}

	Ok(())
}

fn format_time(mut time: u64) -> String {
	let mut out = String::new();

	let hours = time / 60;
	time %= 60;

	let minutes = time;

	if hours > 0 {
		out += &format!("{hours}h ");
	}

	out += &format!("{minutes}m");

	out
}

/// The stored stats data
#[derive(Serialize, Deserialize, Clone, Default)]
struct Stats {
	/// The instances with stored stats
	instances: HashMap<String, InstanceStats>,
}

impl Stats {
	fn open() -> anyhow::Result<Self> {
		let path = Self::get_path();
		if path.exists() {
			serde_json::from_reader(File::open(path)?).context("Failed to read stats from file")
		} else {
			let out = Self::default();
			serde_json::to_writer(File::create(path)?, &out)?;
			Ok(out)
		}
	}

	fn write(self) -> anyhow::Result<()> {
		let path = Self::get_path();
		serde_json::to_writer(File::create(path)?, &self)?;
		Ok(())
	}

	fn get_path() -> PathBuf {
		get_data_dir().join("internal").join("stats.json")
	}
}

/// Stats for a single instance
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
struct InstanceStats {
	/// The playtime of the instance in minutes
	playtime: u64,
	/// The number of times the instance has been launched
	launches: u32,
	/// The last launch time of the instance
	last_launch: Option<u64>,
}

impl InstanceStats {
	/// Calculate playtime
	fn calculate_playtime(&self) -> u64 {
		// Due to the minutes always rounding down every time we stop, there will be a constant
		// integration error of on average half a minute every launch. Counteract this by adding back that
		// half a minute for every launch
		self.playtime + self.launches as u64 / 2
	}
}

/// Config for the plugin
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
struct Config {
	/// Whether to track stats while the instance is running
	#[serde(default = "default_live_tracking")]
	live_tracking: bool,
}

fn default_live_tracking() -> bool {
	true
}

/// Gets the formatted stat card HTML for the given stats
fn format_stat_card(stats: &InstanceStats) -> String {
	let out = include_str!("stat_card.html");

	let out = out.replace("{{playtime}}", &format_time(stats.calculate_playtime()));

	let last_launch = get_last_launch_difference(stats.last_launch).unwrap_or("Never".into());

	out.replace("{{last_played}}", &last_launch)
}

fn get_last_launch_difference(last_launch: Option<u64>) -> Option<String> {
	let last_launch = last_launch?;
	let now = utc_timestamp().ok()?;

	let last_launch = DateTime::from_timestamp_secs(last_launch as i64)?;
	let now = DateTime::from_timestamp_secs(now as i64)?;

	let diff = now - last_launch;

	Some(format_time_large(diff.num_minutes() as u64) + " ago")
}

/// Formats a larger time in minutes
fn format_time_large(mut time: u64) -> String {
	let days = time / 24 / 60;
	time %= 24 * 60;

	let hours = time / 60;
	time %= 60;

	let minutes = time;

	if days > 0 {
		format!("{days} days")
	} else if hours > 0 {
		format!("{hours} hours")
	} else {
		format!("{minutes} minutes")
	}
}
