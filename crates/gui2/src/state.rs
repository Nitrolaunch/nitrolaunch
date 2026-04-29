use nitrolaunch::{config::Config, io::paths::Paths, plugin::PluginManager, shared::output::NoOp};
use reqwest::Client;
use tokio::sync::{broadcast, watch};

use crate::{components::nav::router::Page, event::AppEvent, secrets::get_ms_client_id};

#[derive(Clone)]
pub struct AppState {
	event_tx: broadcast::Sender<AppEvent>,
	route_tx: watch::Sender<Page>,
	pub paths: Paths,
	pub client: Client,
}

impl AppState {
	pub fn new() -> Self {
		let (event_tx, _) = broadcast::channel(50);
		let (route_tx, _) = watch::channel(Page::Home);

		let paths = Paths::new_no_create().unwrap();

		Self {
			event_tx,
			route_tx,
			paths,
			client: Client::new(),
		}
	}

	pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
		self.event_tx.subscribe()
	}

	pub fn subscribe_route(&self) -> watch::Receiver<Page> {
		self.route_tx.subscribe()
	}

	pub fn set_route(&self, route: Page) {
		let _ = self.route_tx.send(route);
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
