use std::sync::Arc;

use freya::radio::RadioChannel;
use nitrolaunch::{config::Config, io::paths::Paths, plugin::PluginManager, shared::output::NoOp};
use reqwest::Client;
use tokio::sync::broadcast;

use crate::{
	components::nav::router::Page, event::AppEvent, secrets::get_ms_client_id, theme::Theme,
};

#[derive(Clone)]
pub struct AppState {
	event_tx: broadcast::Sender<AppEvent>,
	theme: Arc<Theme>,
	route: Page,
	pub paths: Paths,
	pub client: Client,
}

#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum AppChannel {
	Default,
	Route,
	Theme,
}

impl RadioChannel<AppState> for AppChannel {}

impl AppState {
	pub fn new() -> Self {
		let (event_tx, _) = broadcast::channel(50);

		let paths = Paths::new_no_create().unwrap();

		Self {
			event_tx,
			theme: Arc::new(Theme::dark()),
			route: Page::Home,
			paths,
			client: Client::new(),
		}
	}

	pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
		self.event_tx.subscribe()
	}

	pub fn theme(&self) -> Arc<Theme> {
		self.theme.clone()
	}

	pub fn route(&self) -> Page {
		self.route.clone()
	}

	pub fn navigate(&mut self, route: Page) {
		self.route = route;
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
