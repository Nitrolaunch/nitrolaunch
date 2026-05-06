use std::{rc::Rc, sync::Arc};

use anyhow::Context;
use freya::{
	prelude::use_consume,
	radio::{RadioChannel, RadioStation, use_radio},
};
use nitrolaunch::{
	config::Config, instance::tracking::RunningInstanceEntry, io::paths::Paths,
	plugin::PluginManager, shared::output::NoOp,
};
use reqwest::Client;
use tokio::sync::broadcast;

use crate::{
	components::footer::FooterItem,
	instance_manager::RunningInstanceManager,
	routing::{Navigator, Page},
	secrets::get_ms_client_id,
	theme::Theme,
	util::Shared,
};

/// Global state for frontend / UI related things. Only usable on the freya thread.
#[derive(Clone)]
pub struct FrontState {
	theme: Arc<Theme>,
	navigator: Navigator,
	radio: RadioStation<(), FrontChannel>,
	footer: FooterItem,
	event_rx: Rc<broadcast::Receiver<BackEvent>>,
}

/// Different "channels" for listening to changes in parts of the global frontend state
#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum FrontChannel {
	/// Changes to the route
	Route,
	/// Changes to the footer item
	FooterItem,
	/// Changes to the theme
	Theme,
}

impl RadioChannel<()> for FrontChannel {}

impl FrontState {
	pub fn new(
		radio: RadioStation<(), FrontChannel>,
		event_rx: broadcast::Receiver<BackEvent>,
	) -> Self {
		Self {
			theme: Arc::new(Theme::dark_minimal()),
			navigator: Navigator::new(),
			radio,
			footer: FooterItem::None,
			event_rx: Rc::new(event_rx),
		}
	}

	/// Subscribe to changes in the front state on the given channel for this component, re-rendering when it updates
	pub fn subscribe(&self, channel: FrontChannel) {
		use_radio(channel).read();
	}

	pub fn subscribe_events(&self) -> broadcast::Receiver<BackEvent> {
		self.event_rx.resubscribe()
	}

	fn invalidate(&self, channel: FrontChannel) {
		self.radio.clone().write_channel(channel);
	}

	pub fn theme(&self) -> Arc<Theme> {
		self.theme.clone()
	}

	pub fn route(&self) -> &Page {
		self.navigator.route()
	}

	pub fn navigate(&mut self, route: Page) {
		let prev_route = self.navigator.route().clone();
		self.navigator.navigate(route);
		self.check_route_change(prev_route);
		self.invalidate(FrontChannel::Route);
	}

	pub fn forward(&mut self) {
		let prev_route = self.navigator.route().clone();
		self.navigator.forward();
		self.check_route_change(prev_route);
		self.invalidate(FrontChannel::Route);
	}

	pub fn back(&mut self) {
		let prev_route = self.navigator.route().clone();
		self.navigator.back();
		self.check_route_change(prev_route);
		self.invalidate(FrontChannel::Route);
	}

	pub fn can_go_forward(&self) -> bool {
		self.navigator.can_go_forward()
	}

	pub fn can_go_back(&self) -> bool {
		self.navigator.can_go_back()
	}

	fn check_route_change(&mut self, prev_route: Page) {
		if prev_route == Page::Home && self.navigator.route() != &Page::Home {
			self.footer = FooterItem::None;
			self.invalidate(FrontChannel::FooterItem);
		}
	}

	pub fn set_footer(&mut self, item: FooterItem) {
		self.footer = item;
		self.invalidate(FrontChannel::FooterItem);
	}

	pub fn footer(&self) -> &FooterItem {
		&self.footer
	}
}

/// Gives access to front state
pub fn use_front_state() -> Shared<FrontState> {
	use_consume()
}

/// Global state for Nitrolaunch-related things. Thread-safe, can be passed to tokio tasks.
#[derive(Clone)]
pub struct BackState {
	pub event_tx: broadcast::Sender<BackEvent>,
	pub paths: Paths,
	pub client: Client,
	pub plugins: PluginManager,
	pub running_instances: RunningInstanceManager,
}

impl BackState {
	pub async fn new(event_tx: broadcast::Sender<BackEvent>) -> anyhow::Result<Self> {
		let paths = Paths::new_no_create()?;
		let plugins = PluginManager::load(&paths, &mut NoOp).await?;

		let running_instances = RunningInstanceManager::new(&paths, event_tx.clone())
			.context("Failed to create running instance manager")?;

		tokio::spawn(running_instances.clone().get_run_task());

		Ok(Self {
			event_tx,
			paths,
			plugins,
			client: Client::new(),
			running_instances,
		})
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

/// Events sent from the backend
#[derive(Clone)]
pub enum BackEvent {
	UpdateRunningInstances,
}
