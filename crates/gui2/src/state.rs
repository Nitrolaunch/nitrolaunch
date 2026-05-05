use std::sync::Arc;

use freya::radio::RadioChannel;
use nitrolaunch::{config::Config, io::paths::Paths, plugin::PluginManager, shared::output::NoOp};
use reqwest::Client;

use crate::{routing::Navigator, secrets::get_ms_client_id, theme::Theme};

#[derive(Clone)]
pub struct AppState {
	theme: Arc<Theme>,
	pub navigator: Navigator,
	pub paths: Paths,
	pub client: Client,
	pub plugins: PluginManager,
}

/// Different "channels" for listening to changes in parts of the global state
#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum AppChannel {
	/// Assorted random stuff
	Default,
	/// Changes to the route
	Route,
	/// Changes to configuration
	Config,
	/// Changes to the footer item
	FooterItem,
	/// Changes to the theme
	Theme,
}

impl RadioChannel<AppState> for AppChannel {}

impl AppState {
	pub async fn new() -> anyhow::Result<Self> {
		let paths = Paths::new_no_create()?;
		let plugins = PluginManager::load(&paths, &mut NoOp).await?;

		Ok(Self {
			theme: Arc::new(Theme::dark()),
			navigator: Navigator::new(),
			paths,
			plugins,
			client: Client::new(),
		})
	}

	pub fn theme(&self) -> Arc<Theme> {
		self.theme.clone()
	}

	pub async fn config(&self) -> anyhow::Result<Config> {
		let paths = self.paths.clone();
		let plugins = self.plugins.clone();

		tokio::spawn(async move {
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
