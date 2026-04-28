use std::sync::Arc;

use tokio::sync::broadcast;

use crate::{components::nav::router::Page, event::AppEvent};

#[derive(Clone)]
pub struct AppState {
	event_tx: broadcast::Sender<AppEvent>,
	event_rx: Arc<broadcast::Receiver<AppEvent>>,
}

impl AppState {
	pub fn new() -> Self {
		let (event_tx, event_rx) = broadcast::channel(25);

		Self {
			event_tx,
			event_rx: Arc::new(event_rx),
		}
	}

	pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
		self.event_rx.resubscribe()
	}

	pub fn set_route(&self, route: Page) {
		let _ = self.event_tx.send(AppEvent::RouteChanged(route));
	}
}
