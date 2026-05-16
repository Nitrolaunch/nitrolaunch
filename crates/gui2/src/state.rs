use std::{collections::HashMap, rc::Rc, sync::Arc};

use anyhow::Context;
use freya::{
	prelude::use_consume,
	radio::{RadioChannel, RadioStation, use_radio},
};
use nitrolaunch::{
	config::Config,
	io::{logging::Logger, paths::Paths},
	plugin::PluginManager,
	shared::{
		output::{Message, MessageContents, NoOp},
		pkg::{PackageDiff, ResolutionError},
	},
};
use reqwest::Client;
use tokio::sync::{Mutex, broadcast, mpsc};

use crate::{
	components::footer::FooterItem,
	instance_manager::RunningInstanceManager,
	ops::task::TaskManager,
	output::{LauncherOutput, OutputInner},
	pages::instance::config::ConfiguredItem,
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
	configured_item: Option<ConfiguredItem>,
	event_rx: Rc<broadcast::Receiver<BackEvent>>,
}

/// Different "channels" for listening to changes in parts of the global frontend state
#[derive(PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum FrontChannel {
	/// Changes to the route
	Route,
	/// Changes to the footer item
	FooterItem,
	/// Changes to the configured item
	ConfiguredItem,
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
			configured_item: None,
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

	pub fn set_configured_item(&mut self, item: Option<ConfiguredItem>) {
		self.configured_item = item;
		self.invalidate(FrontChannel::ConfiguredItem);
	}

	pub fn configured_item(&self) -> Option<&ConfiguredItem> {
		self.configured_item.as_ref()
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
	output_inner: OutputInner,
	task_manager: Arc<Mutex<TaskManager>>,
}

impl BackState {
	pub async fn new(event_tx: broadcast::Sender<BackEvent>) -> anyhow::Result<Self> {
		let paths = Paths::new_no_create()?;
		let plugins = PluginManager::load(&paths, &mut NoOp).await?;

		let running_instances = RunningInstanceManager::new(&paths, event_tx.clone())
			.context("Failed to create running instance manager")?;

		tokio::spawn(running_instances.clone().get_run_task());

		let (logger_tx, mut logger_rx) = mpsc::channel::<Message>(25);
		let mut logger = Logger::new(&paths, "gui").context("Failed to set up logger")?;
		tokio::spawn(async move {
			if let Some(message) = logger_rx.recv().await {
				let _ = logger.log_message(message.contents, message.level);
			}
		});

		let task_manager = Arc::new(Mutex::new(TaskManager::new(event_tx.clone())));
		tokio::spawn(TaskManager::get_run_task(task_manager.clone()));

		Ok(Self {
			output_inner: OutputInner {
				event_tx: event_tx.clone(),
				password_prompt: Arc::new(Mutex::new(None)),
				yes_no_prompt: Arc::new(Mutex::new(None)),
				passkeys: Arc::new(Mutex::new(HashMap::new())),
				logger: logger_tx,
			},
			task_manager,
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

	pub fn output(&self) -> LauncherOutput {
		LauncherOutput::new(&self.output_inner)
	}

	pub fn register_task(&self, task_id: &str, task: tokio::task::JoinHandle<anyhow::Result<()>>) {
		let manager = self.task_manager.clone();
		let task_id = task_id.to_string();
		tokio::spawn(async move { manager.lock().await.register_task(task_id, task) });
	}
}

/// Events sent from the backend
#[derive(Clone)]
pub enum BackEvent {
	OutputMessage {
		message: MessageContents,
		task: Option<String>,
	},
	OutputStartTask(String),
	OutputEndTask(String),
	OutputEndProcess(Option<String>),
	OutputEndSection(Option<String>),
	OutputResolutionError {
		error: Arc<ResolutionError>,
		instance_id: String,
	},
	UpdateRunningInstances,
	ShowAuthPrompt {
		url: String,
		device_code: String,
	},
	CloseAuthPrompt,
	ShowYesNoPrompt {
		message: String,
	},
	ShowPasskeyPrompt,
	ShowPackageDiffsPrompt {
		diffs: Vec<PackageDiff>,
	},
}
