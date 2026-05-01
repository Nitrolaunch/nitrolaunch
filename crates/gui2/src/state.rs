use freya::radio::RadioChannel;
use nitrolaunch::{config::Config, io::paths::Paths, plugin::PluginManager, shared::output::NoOp};
use reqwest::Client;
use tokio::sync::broadcast;

use crate::{event::AppEvent, secrets::get_ms_client_id};

#[derive(Clone)]
pub struct AppState {
	event_tx: broadcast::Sender<AppEvent>,
	pub paths: Paths,
	pub client: Client,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum AppChannel {
	Default,
}

impl RadioChannel<AppState> for AppChannel {}

impl AppState {
	pub fn new() -> Self {
		let (event_tx, _) = broadcast::channel(50);

		let paths = Paths::new_no_create().unwrap();

		Self {
			event_tx,
			paths,
			client: Client::new(),
		}
	}

	pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
		self.event_tx.subscribe()
	}

	pub async fn config(&self) -> anyhow::Result<Config> {
		let paths = self.paths.clone();

		tokio::spawn(async move {
			let plugins = PluginManager::load(&paths, &mut NoOp).await?;

			Config::load(
				&Config::get_path(&paths),
				plugins,
				false,
				&paths,
				get_ms_client_id(),
				&mut NoOp,
			)
			.await
		})
		.await?
	}
}
