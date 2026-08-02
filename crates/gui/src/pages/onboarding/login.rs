use crate::{
	components::{account::auth::ms_auth_info, misc::status_panel},
	icons::microsoft_icon,
	ops::{account::OnboardAccount, task::Task},
	pages::onboarding::banner_image,
	prelude::*,
	state::BackEvent,
	util::assets::SPLASH4,
};

#[derive(PartialEq)]
pub struct LoginTab;

impl Component for LoginTab {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let login_mutation = use_mutation(Mutation::new(OnboardAccount::new(back_state.clone())));

		let step = use_state(|| AuthStep::NotStarted);

		let front_state2 = front_state.clone();
		let mut step2 = step.clone();
		use_future(move || {
			let front_state2 = front_state2.clone();
			async move {
				let mut event_rx = front_state2.read().subscribe_events();
				while let Ok(event) = event_rx.recv().await {
					match event {
						BackEvent::ShowAuthPrompt { url, device_code } => {
							step2.set(AuthStep::InProgress { url, device_code });
						}
						BackEvent::OutputEndTask {
							task: Task::LoginFirstAccount,
							success,
						} => {
							if success {
								step2.set(AuthStep::Completed);
							} else {
								step2.set(AuthStep::NotStarted);
							}
						}
						_ => {}
					}
				}
			}
		});

		let contents = match &*step.read() {
			AuthStep::NotStarted => elem_text_button(
				microsoft_icon(theme.primary),
				"Login with Microsoft",
				&theme,
			)
			.width(Size::px(180.0))
			.active(&theme)
			.on_press(move |_| {
				login_mutation.mutate(());
			})
			.into_element(),
			AuthStep::InProgress { url, device_code } => rect()
				.width(Size::px(400.0))
				.height(Size::px(400.0))
				.child(ms_auth_info(url.clone(), device_code.clone(), &theme))
				.into_element(),
			AuthStep::Completed => status_panel(
				"Logged in! You can add more accounts at any time from the settings",
				theme.success,
				&theme,
			)
			.width(Size::px(400.0))
			.into_element(),
		};

		let left = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.center()
			.spacing(theme.gap3)
			.padding(theme.gap3)
			.child(
				label()
					.text("Almost there...")
					.font_size(24.0)
					.font_weight(FontWeight::BOLD),
			)
			.child(
				label()
					.text("Let's get your account connected")
					.text_align(TextAlign::Center)
					.width(Size::px(300.0))
					.color(theme.fg2),
			)
			.child(contents);

		rect()
			.expanded()
			.flex()
			.horizontal()
			.child(left)
			.child(banner_image(SPLASH4, "splash4", false, &theme))
	}
}

enum AuthStep {
	NotStarted,
	InProgress { url: String, device_code: String },
	Completed,
}
