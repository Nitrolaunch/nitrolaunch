use std::path::Path;

use anyhow::{bail, Context};
use clap::Parser;
use lnk::ShellLink;
use nitro_plugin::{
	api::wasm::{
		nitro::get_instances,
		sys::{get_data_dir, get_home_dir, get_os_string},
		WASMPlugin,
	},
	nitro_wasm_plugin,
};

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
		let cli = Cli::try_parse_from(it)?;

		let instances = get_instances().context("Instances not available")?;
		let instance = instances
			.get(&cli.instance)
			.context("Failed to fetch instance")?
			.context("Instance does not exist")?;

		let extension = match get_os_string().as_str() {
			"linux" => "desktop",
			"windows" => "lnk",
			_ => bail!("Shortcuts are not supported on this system"),
		};

		let name = if let Some(name) = cli.name {
			name.trim_end_matches(extension).to_string()
		} else {
			// Generate from parameters
			let mut name = if let Some(name) = instance.name {
				name
			} else {
				cli.instance.clone()
			};

			if let Some(account) = &cli.account {
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

		match get_os_string().as_str() {
			"linux" => {
				let icon = include_bytes!("../../../gui/src-tauri/icons/icon_1024x1024.png");
				let icon_path =
					home_dir.join(".local/share/icons/hicolor/1024x1024/nitrolaunch.png");
				if let Some(parent) = icon_path.parent() {
					let _ = std::fs::create_dir_all(parent);
				}
				let _ = std::fs::write(icon_path, icon);

				let contents = create_linux_shortcut(
					&name,
					&executable_path,
					cli.account.as_deref(),
					cli.quick_play.as_deref(),
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
					cli.account.as_deref(),
					cli.quick_play.as_deref(),
				)?;
			}
			_ => {}
		}

		Ok(())
	})?;

	Ok(())
}

#[derive(clap::Parser)]
struct Cli {
	/// The instance to create a shortcut to
	instance: String,
	/// The name of the shortcut. Defaults to the instance name.
	#[arg(short, long)]
	name: Option<String>,
	/// An optional account to launch the instance with
	#[arg(short, long)]
	account: Option<String>,
	/// An optional world, server, or realm to launch with. Format is world:<world>, server:<ip>, or realm:<realm>
	#[arg(short, long)]
	quick_play: Option<String>,
}

fn create_linux_shortcut(
	base_name: &str,
	exec: &Path,
	account: Option<&str>,
	quick_play: Option<&str>,
) -> String {
	let exec_name = exec.file_name().unwrap().to_string_lossy().to_string();
	let exec = exec.to_string_lossy().to_string();

	let mut args = String::new();
	if let Some(account) = account {
		args += &format!(" --account {account}");
	}
	if let Some(quick_play) = quick_play {
		args += &format!(" --quick-play {quick_play}");
	}

	return format!(
		r#"[Desktop Entry]
Type=Application
Version=1.0
Name=Launch {base_name}
Comment=Nitrolaunch instance
Path={exec}
Exec={exec_name}{args}
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
	if let Some(account) = account {
		args += &format!(" --account {account}");
	}
	if let Some(quick_play) = quick_play {
		args += &format!(" --quick-play {quick_play}");
	}
	link.set_arguments(Some(args).filter(|x| !x.is_empty()));

	link.save(shortcut_path).context("Failed to save shortcut")
}
