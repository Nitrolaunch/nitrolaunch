use std::collections::HashMap;

use nitrolaunch::shared::output::MessageContents;

use crate::{prelude::*, state::BackEvent};

#[derive(PartialEq)]
pub struct OutputIndicator;

impl Component for OutputIndicator {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();

		let mut tasks = use_state::<HashMap<String, Task>>(|| HashMap::new());
		let mut is_open = use_state(|| false);

		use_side_effect(move || {
			if tasks.read().is_empty() {
				is_open.set(false);
			}
		});

		let event_rx = front_state.read().subscribe_events();
		use_future(move || {
			let mut event_rx = event_rx.resubscribe();

			async move {
				loop {
					let Ok(ev) = event_rx.recv().await else {
						continue;
					};
					match ev {
						BackEvent::OutputStartTask(task) => {
							if !tasks.read().contains_key(&task) {
								tasks.write().insert(task, Task::new());
							}
						}
						BackEvent::OutputEndTask(task) => {
							tasks.write().remove(&task);
						}
						BackEvent::OutputMessage {
							message,
							task: Some(task),
						} => {
							if let Some(task) = tasks.write().get_mut(&task) {
								match message {
									MessageContents::StartProcess(msg) => task.process = Some(msg),
									MessageContents::Success(..) => task.process = None,
									MessageContents::Header(header) => task.section = Some(header),
									other => task.messages.push(other),
								}
							}
						}
						BackEvent::OutputEndProcess(Some(task)) => {
							if let Some(task) = tasks.write().get_mut(&task) {
								task.process = None;
							}
						}
						BackEvent::OutputEndSection(Some(task)) => {
							if let Some(task) = tasks.write().get_mut(&task) {
								task.section = None;
							}
						}
						_ => {}
					}
				}
			}
		});

		let indicator_text = match tasks.read().len() {
			0 => "No tasks running".into(),
			1 => tasks.read().iter().next().unwrap().0.clone(),
			other => format!("{other} tasks running"),
		};

		let indicator = rect()
			.width(Size::fill())
			.height(Size::px(36.0))
			.item_colorway(&theme, false, false)
			.background(theme.bg)
			.corner_radius(theme.round2)
			.center()
			.on_press(move |_| is_open.toggle())
			.child(indicator_text);

		let popout = if *is_open.read() {
			Some(
				rect()
					.width(Size::fill())
					.height(Size::px(128.0))
					.item_colorway(&theme, false, false)
					.corner_radius(theme.round2)
					.margin((0.0, 0.0, 8.0, 0.0)),
			)
		} else {
			None
		};

		Attached::new(indicator).top().maybe_child(popout)
	}
}

struct Task {
	messages: Vec<MessageContents>,
	process: Option<String>,
	section: Option<String>,
}

impl Task {
	fn new() -> Self {
		Self {
			messages: Vec::new(),
			process: None,
			section: None,
		}
	}
}
