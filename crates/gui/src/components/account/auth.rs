use crate::{
	components::{dialog::modal::Modal, icon_text_button},
	ops::task::{KillTask, Task},
	prelude::*,
};
use nitrolaunch::shared::util::open_link;

#[derive(PartialEq)]
pub struct MicrosoftAuthPrompt {
	url: String,
	device_code: String,
	on_close: EventHandler<()>,
}

impl MicrosoftAuthPrompt {
	pub fn new(
		url: impl Into<String>,
		device_code: impl Into<String>,
		on_close: impl Into<EventHandler<()>>,
	) -> Self {
		Self {
			url: url.into(),
			device_code: device_code.into(),
			on_close: on_close.into(),
		}
	}
}

impl Component for MicrosoftAuthPrompt {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let kill_task_mutation = use_mutation(Mutation::new(KillTask::new(back_state.clone())));

		let body = ms_auth_info(self.url.clone(), self.device_code.clone(), &theme);

		let on_close = self.on_close.clone();
		Modal::new("Microsoft Authentication".into(), "lock".into())
			.on_close(move |_| {
				on_close.call(());
				kill_task_mutation.mutate(Task::LoginAccount);
			})
			.maybe_child(true, move || body)
			.cancel_button()
	}
}

pub fn ms_auth_info(url: String, device_code: String, theme: &Theme) -> Rect {
	let code_copy = button(theme)
		.child(device_code.clone())
		.on_press(move |_| {
			let _ = Clipboard::set(device_code.clone());
		});
	let code_copy = field("Copy this code", "copy", theme, code_copy);

	let url2 = url.clone();
	let button = icon_text_button("globe", "Open login page", theme)
		.color(theme.primary)
		.border_fill(theme.primary)
		.background(theme.primary_bg)
		.hover_background(theme.primary_bg)
		.on_press(move |_| {
			let _ = open_link(&url2);
		});
	let open_login_page = field(
		"Then paste the code into the login page",
		"globe",
		theme,
		button,
	);

	let fallback = field(
		"If the page doesn't open automatically, you can use the browser link below.",
		"link",
		theme,
		label().text(url).color(theme.fg3),
	);

	rect()
		.expanded()
		.padding(theme.gap2)
		.child(code_copy)
		.child(open_login_page)
		.child(fallback)
}
