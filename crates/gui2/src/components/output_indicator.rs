use std::collections::HashMap;

use nitrolaunch::shared::output::MessageContents;

use crate::{
	ops::task::{KillTask, Task},
	prelude::*,
	state::BackEvent,
};

#[derive(PartialEq)]
pub struct OutputIndicator;

impl Component for OutputIndicator {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let kill_task = use_mutation(Mutation::new(KillTask::new(back_state.clone())));

		let mut tasks = use_state::<HashMap<Task, TaskData>>(|| HashMap::new());
		let mut is_open = use_state(|| false);

		use_side_effect(move || {
			if tasks.read().is_empty() {
				is_open.set(false);
			}
		});

		let front_state2 = front_state.clone();
		use_future(move || {
			let front_state2 = front_state2.clone();
			async move {
				let mut event_rx = front_state2.read().subscribe_events();
				loop {
					let Ok(ev) = event_rx.recv().await else {
						continue;
					};
					match ev {
						BackEvent::OutputStartTask(task) => {
							if !tasks.read().contains_key(&task) {
								tasks.write().insert(task, TaskData::new());
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

		let temp = tasks.read();
		let current_task = temp.iter().next().map(|x| x.0.clone());

		let indicator_text = match tasks.read().len() {
			0 => "No tasks running".into(),
			1 => current_task.as_ref().unwrap().to_string(),
			other => format!("{other} tasks running"),
		};

		let current_task2 = current_task.clone();
		let indicator = rect()
			.width(Size::fill())
			.height(Size::px(36.0))
			.panel_colorway(&theme, false, !tasks.read().is_empty())
			.background(theme.bg)
			.corner_radius(theme.round2)
			.center()
			.maybe(current_task.is_some_and(|x| x.can_cancel()), |this| {
				this.tip(&front_state, "Click to cancel")
			})
			.on_press(move |_| kill_task.mutate(current_task2.clone().unwrap()))
			.child(indicator_text);

		let popout = if *is_open.read() {
			Some(
				rect()
					.width(Size::fill())
					.height(Size::px(128.0))
					.panel_colorway(&theme, false, false)
					.corner_radius(theme.round2)
					.margin((0.0, 0.0, 8.0, 0.0)),
			)
		} else {
			None
		};

		Attached::new(indicator).top().maybe_child(popout)
	}
}

struct TaskData {
	messages: Vec<MessageContents>,
	process: Option<String>,
	section: Option<String>,
}

impl TaskData {
	fn new() -> Self {
		Self {
			messages: Vec::new(),
			process: None,
			section: None,
		}
	}
}
