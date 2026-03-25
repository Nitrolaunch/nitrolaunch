use std::path::Path;

use anyhow::{bail, Context};
use clap::Parser;
use lnk::ShellLink;
use nitro_plugin::{
	api::wasm::{
		nitro::get_instances,
		output::WASMPluginOutput,
		sys::{get_data_dir, get_home_dir, get_os_string},
		WASMPlugin,
	},
	nitro_wasm_plugin,
};
use nitro_shared::output::{MessageContents, NitroOutput};
use serde::Deserialize;

nitro_wasm_plugin!(main, "shortcut");

fn main(plugin: &mut WASMPlugin) -> anyhow::Result<()> {
	plugin.subcommand(|arg| {
		let Some(subcommand) = arg.args.first() else {
			return Ok(());
		};
		if subcommand != "shortcut" {
			return Ok(());
		}

		// Trick the parser to give it the right bin name
		let it =
			std::iter::once("nitro instance shortcut".into()).chain(arg.args.into_iter().skip(1));
		let cli = Settings::try_parse_from(it)?;

		create_shortcut(cli.instance, cli.name, cli.account, cli.quick_play)?;

		WASMPluginOutput::new().display(MessageContents::Success("Shortcut created".into()));
		Ok(())
	})?;

	plugin.custom_action(|arg| {
		if arg.id != "create_shortcut" {
			return Ok(serde_json::Value::Null);
		}

		let settings: Settings =
			serde_json::from_value(arg.payload).context("Failed to deserialize argument")?;

		create_shortcut(
			settings.instance,
			settings.name,
			settings.account,
			settings.quick_play,
		)?;

		WASMPluginOutput::new().display(MessageContents::Success("Shortcut created".into()));

		Ok(serde_json::Value::Null)
	})?;

	Ok(())
}

fn create_shortcut(
	instance_id: String,
	name: Option<String>,
	account: Option<String>,
	quick_play: Option<String>,
) -> anyhow::Result<()> {
	let instances = get_instances().context("Instances not available")?;
	let instance = instances
		.get(&instance_id)
		.context("Failed to fetch instance")?
		.context("Instance does not exist")?;

	let extension = match get_os_string().as_str() {
		"linux" => "desktop",
		"windows" => "lnk",
		_ => bail!("Shortcuts are not supported on this system"),
	};

	let name = if let Some(name) = name {
		name.trim_end_matches(extension).to_string()
	} else {
		// Generate from parameters
		let mut name = if let Some(name) = instance.name {
			name
		} else {
			instance_id.clone()
		};

		if let Some(account) = &account {
			match get_os_string().as_str() {
				"windows" => name = format!("{name} - {account}"),
				_ => name = format!("{name}_{account}"),
			}
		}

		name
	};

	let filename = format!("{name}.{extension}");

	let home_dir = get_home_dir();
	let folder = match get_os_string().as_str() {
		"linux" => home_dir.join(".local/share/applications"),
		"windows" | "macos" => home_dir.join("Desktop"),
		_ => bail!("Unsupported operating system"),
	};

	let path = folder.join(filename);
	let _ = std::fs::create_dir_all(folder);

	// Figure out executable
	let executable_name = match get_os_string().as_str() {
		"linux" | "macos" => "launch_instance.sh",
		"windows" => "launch_instance.bat",
		_ => bail!("Unsupported operating system"),
	};
	let executable_path = get_data_dir().join("internal").join(executable_name);

	if !executable_path.exists() {
		println!("Warning: Nitro executable does not exist");
	}

	let wrapper = instance
		.plugin_config
		.get("shortcut_wrapper")
		.and_then(|x| x.as_str());

	match get_os_string().as_str() {
		"linux" => {
			let icon = include_bytes!("../../../gui/src-tauri/icons/icon_128x128.png");
			let icon_path =
				home_dir.join(".local/share/icons/hicolor/128x128/apps/nitrolaunch.png");
			if let Some(parent) = icon_path.parent() {
				let _ = std::fs::create_dir_all(parent);
			}
			let _ = std::fs::write(icon_path, icon);

			let contents = create_linux_shortcut(
				&name,
				&executable_path,
				wrapper,
				&instance_id,
				account.as_deref(),
				quick_play.as_deref(),
			);

			std::fs::write(path, contents).context("Failed to write shortcut")?;
		}
		"windows" => {
			let icon = include_bytes!("../../../gui/src-tauri/icons/icon.ico");
			let icon_path = get_data_dir().join("internal/shortcut_icon.ico");
			let _ = std::fs::write(&icon_path, icon);

			create_windows_shortcut(
				&path,
				&name,
				&executable_path,
				&icon_path,
				&instance_id,
				account.as_deref(),
				quick_play.as_deref(),
			)?;
		}
		_ => {}
	}

	Ok(())
}

#[derive(clap::Parser, Deserialize)]
struct Settings {
	/// The instance to create a shortcut to
	instance: String,
	/// The name of the shortcut. Defaults to the instance name.
	#[arg(short, long)]
	#[serde(default)]
	name: Option<String>,
	/// An optional account to launch the instance with
	#[arg(short, long)]
	#[serde(default)]
	account: Option<String>,
	/// An optional world, server, or realm to launch with. Format is world:<world>, server:<ip>, or realm:<realm>
	#[arg(short, long)]
	#[serde(default)]
	quick_play: Option<String>,
}

fn create_linux_shortcut(
	base_name: &str,
	exec: &Path,
	wrapper: Option<&str>,
	instance_id: &str,
	account: Option<&str>,
	quick_play: Option<&str>,
) -> String {
	let exec = exec.to_string_lossy().to_string();

	let mut args = String::new();
	args += instance_id;
	if let Some(account) = account {
		args += &format!(" --account {account}");
	}
	if let Some(quick_play) = quick_play {
		args += &format!(" --quick-play {quick_play}");
	}

	let command = format!("{exec} {args}");
	let command = if let Some(wrapper) = wrapper {
		wrapper.replace("$cmd", &command)
	} else {
		command
	};

	return format!(
		r#"[Desktop Entry]
Type=Application
Version=1.0
Name=Launch {base_name}
Comment=Nitrolaunch instance
Exec={command}
Icon=nitrolaunch
Terminal=false
Categories=Games;	
"#
	);
}

fn create_windows_shortcut(
	shortcut_path: &Path,
	base_name: &str,
	exec: &Path,
	icon_path: &Path,
	instance_id: &str,
	account: Option<&str>,
	quick_play: Option<&str>,
) -> anyhow::Result<()> {
	// We have to construct the link ourselves since the default impl uses the WASM-unsupported fs::canonicalize
	let mut link = ShellLink::default();
	link.set_relative_path(Some(format!(
		".\\{}",
		exec.file_name().unwrap().to_str().unwrap()
	)));
	link.set_working_dir(Some(exec.parent().unwrap().to_string_lossy().to_string()));

	link.set_name(Some(base_name.to_string()));
	link.set_icon_location(Some(icon_path.to_string_lossy().to_string()));

	let mut args = String::new();
	args += instance_id;
	if let Some(account) = account {
		args += &format!(" --account {account}");
	}
	if let Some(quick_play) = quick_play {
		args += &format!(" --quick-play {quick_play}");
	}
	link.set_arguments(Some(args).filter(|x| !x.is_empty()));

	link.save(shortcut_path).context("Failed to save shortcut")
}
