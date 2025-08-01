/// Downloading game assets
pub mod assets;
/// Structure for the client metadata file
pub mod client_meta;
/// Downloading game Java libraries
pub mod libraries;
/// Downloading and using the version manifest
pub mod version_manifest;

use crate::io::files::paths::Paths;
use crate::io::update::UpdateManager;
use nitro_shared::translate;
use nitro_shared::util::cap_first_letter;
use nitro_shared::Side;

use reqwest::Client;

use super::download;

/// Downloading the game JAR file
pub mod game_jar {
	use nitro_shared::output::{MessageContents, MessageLevel, NitroOutput, OutputProcess};

	use self::download::ProgressiveDownload;

	use super::{client_meta::ClientMeta, *};

	/// Downloads the vanilla game JAR file
	pub async fn get(
		side: Side,
		client_meta: &ClientMeta,
		version: &str,
		paths: &Paths,
		manager: &UpdateManager,
		client: &Client,
		o: &mut impl NitroOutput,
	) -> anyhow::Result<()> {
		let side_str = side.to_string();
		let path = crate::io::minecraft::game_jar::get_path(side, version, None, paths);
		if !manager.should_update_file(&path) {
			return Ok(());
		}

		let mut process = OutputProcess::new(o);
		let download_message = translate!(process, StartDownloadingGameJar, "side" = &side_str);
		process.display(
			MessageContents::StartProcess(download_message.clone()),
			MessageLevel::Important,
		);

		let download = match side {
			Side::Client => &client_meta.downloads.client,
			Side::Server => &client_meta.downloads.server,
		};

		let mut download = ProgressiveDownload::file(&download.url, path, client).await?;
		while !download.is_finished() {
			download.poll_download().await?;
			process.display(
				MessageContents::Associated(
					Box::new(download.get_progress()),
					Box::new(MessageContents::Simple(download_message.clone())),
				),
				MessageLevel::Important,
			);
		}

		let side_str = cap_first_letter(&side_str);

		let message = MessageContents::Success(translate!(
			process,
			FinishDownloadingGameJar,
			"side" = &side_str
		));
		process.display(message, MessageLevel::Important);

		Ok(())
	}
}

/// Downloading and using the logging config file
pub mod log_config {
	use std::path::PathBuf;

	use super::{client_meta::ClientMeta, *};

	/// Get the logging configuration file and returns the path to it
	pub async fn get(
		client_meta: &ClientMeta,
		version: &str,
		paths: &Paths,
		manager: &UpdateManager,
		client: &Client,
	) -> anyhow::Result<()> {
		let path = get_path(version, paths);

		if !manager.should_update_file(&path) {
			return Ok(());
		}

		let url = &client_meta.logging.client.file.url;
		download::file(url, &path, client).await?;

		Ok(())
	}

	/// Get the path to the logging config file
	pub fn get_path(version: &str, paths: &Paths) -> PathBuf {
		let version_dir = paths.internal.join("versions").join(version);
		version_dir.join("logging.xml")
	}
}
