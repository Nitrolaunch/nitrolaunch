use nitrolaunch::shared::output::MessageContents;

use crate::{
	components::{
		dialog::modal::{Modal, ModalButton},
		instance::transfer::{MigrateContents, on_migrate},
		misc::{progress_bar, socials, status_panel},
	},
	ops::{
		plugins::InstallDefaultPlugins,
		task::{KillTask, Task},
		transfer::MigrateInstances,
	},
	pages::onboarding::login::LoginTab,
	prelude::*,
	state::{BackEvent, use_launcher_data},
	util::assets::{LOGO_LARGE, SPLASH, SPLASH2, SPLASH3, SPLASH5},
};

mod login;

#[derive(PartialEq)]
pub struct OnboardingModal;

impl Component for OnboardingModal {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let mut data = use_launcher_data();

		let mut tab = use_state(|| Tab::Welcome);

		let contents = match &*tab.read() {
			Tab::Welcome => welcome_tab(&theme).into_element(),
			Tab::Plugins => PluginsTab.into_element(),
			Tab::Migrate => MigrateTab.into_element(),
			Tab::Accounts => LoginTab.into_element(),
			Tab::Finished => done_tab(&theme).into_element(),
		};

		let next_text = if *tab.read() == Tab::Finished {
			"Finish"
		} else {
			"Next"
		};
		let next_icon = if *tab.read() == Tab::Finished {
			"check"
		} else {
			"arrow_right"
		};

		let mut tab2 = tab.clone();
		let front_state2 = front_state.clone();
		Modal::new_no_title()
			.size_xlarge()
			.maybe_button(
				*tab.read() != Tab::Welcome,
				ModalButton {
					title: "Back".into(),
					icon: "arrow_left".into(),
					on_click: EventHandler::new(move |_| {
						if *tab.peek() != Tab::Welcome {
							let index = tab.peek().index();
							tab.set(Tab::from_index(index - 1));
						}
					}),
					active: false,
				},
			)
			.button(ModalButton {
				title: next_text.into(),
				icon: next_icon.into(),
				on_click: EventHandler::new(move |_| {
					if *tab2.peek() != Tab::Finished {
						let index = tab2.peek().index();
						tab2.set(Tab::from_index(index + 1));
					} else {
						data.data.write().launcher_opened_before = true;
						data.save();
						front_state2.write().set_modal(None);
					}
				}),
				active: true,
			})
			.maybe_child(true, || contents)
	}
}

fn welcome_tab(theme: &Theme) -> impl IntoElement {
	let left = banner_image(SPLASH, "splash", true, theme);

	let padding = theme.gap3 * 2.5;

	// 	let features = r#"
	// - **Modular**: Build the launcher you need with the features you want.
	// - **Stable**: Instances only update when you tell them to.
	// "#;

	let right = rect()
		.width(Size::flex(1.0))
		.height(Size::fill())
		.spacing(theme.gap3)
		.padding(Gaps::new(0.0, padding, theme.gap3, padding))
		.cross_align(Alignment::Center)
		.font_size(20.0)
		.child(ImageViewer::new(("logo-large", LOGO_LARGE)).width(Size::fill()))
		.child(
			label()
				.color(theme.fg2)
				.text("Let's get you set up. This will only take a few minutes."),
		)
		// .child(
		// 	MarkdownViewer::new(features)
		// 		.width(Size::fill())
		// 		.color(theme.fg2)
		// 		.margin(Gaps::new(theme.gap3, 0.0, 0.0, 0.0)),
		// )
		.child(
			rect()
				.expanded()
				.main_align(Alignment::End)
				.spacing(theme.gap3)
				.child(
					status_panel(
						"Keep in mind the launcher is still in beta. There's lots of features on the horizon!",
						theme.panel,
						&theme,
					)
					.width(Size::fill()),
				)
				.child(socials(&theme)),
		);

	rect()
		.expanded()
		.flex()
		.horizontal()
		.child(left)
		.child(right)
}

#[derive(PartialEq)]
struct PluginsTab;

impl Component for PluginsTab {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let install_mutation = use_mutation(Mutation::new(InstallDefaultPlugins::new(
			back_state.clone(),
		)));
		let cancel_task = use_mutation(Mutation::new(KillTask::new(back_state.clone())));

		let progress = use_state(|| (0, 0));

		let front_state2 = front_state.clone();
		let mut progress2 = progress.clone();
		use_future(move || {
			let front_state2 = front_state2.clone();
			async move {
				let mut event_rx = front_state2.read().subscribe_events();
				while let Ok(event) = event_rx.recv().await {
					match event {
						BackEvent::OutputMessage {
							message: MessageContents::Progress { current, total },
							task: Some(Task::InstallDefaultPlugins),
						} => {
							progress2.set((current, total));
						}
						BackEvent::OutputEndTask {
							task: Task::InstallDefaultPlugins,
							..
						} => {
							progress2.set((0, 0));
						}
						_ => {}
					}
				}
			}
		});

		let button = if progress.read().1 == 0 {
			icon_text_button("download", "Install", &theme)
				.width(Size::px(180.0))
				.active(&theme)
				.on_press(move |_| {
					install_mutation.mutate(());
				})
		} else {
			let mut progress2 = progress.clone();
			icon_text_button("delete", "Cancel", &theme)
				.width(Size::px(180.0))
				.active(&theme)
				.on_press(move |_| {
					cancel_task.mutate(Task::InstallDefaultPlugins);
					progress2.set((0, 0));
				})
		};

		let left = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.center()
			.spacing(theme.gap3)
			.child(label().text("Default Plugins").font_size(24.0).font_weight(FontWeight::BOLD))
			.child(label().text("We recommend you install some plugins before you start. These include features like mod repositories, launcher support, and themes").width(Size::px(300.0)).color(theme.fg2))
			.maybe(progress.read().1 > 0, |this| {
				let (current, total) = *progress.read();
				this.child(rect()
					.width(Size::px(300.0))
					.child(
						progress_bar(&theme, current as f32 / total as f32)
							.width(Size::fill())
							.height(Size::px(8.0)),
					))
			})
			.child(button);

		rect()
			.expanded()
			.flex()
			.horizontal()
			.child(left)
			.child(banner_image(SPLASH2, "splash2", false, &theme))
	}
}

#[derive(PartialEq)]
struct MigrateTab;

impl Component for MigrateTab {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let migrate_mutation =
			use_mutation(Mutation::new(MigrateInstances::new(back_state.clone())));

		let format = use_state::<Option<String>>(|| None);
		let link = use_state(|| false);
		let instances = use_state(|| Vec::new());

		let contents = MigrateContents {
			format: format.clone(),
			link: link.clone(),
			instances: instances.clone(),
		};

		let mut on_submit = on_migrate(
			front_state.clone(),
			migrate_mutation,
			format.clone(),
			link.clone(),
			instances.clone(),
		);

		let right = rect()
			.width(Size::flex(1.0))
			.height(Size::fill())
			.flex()
			.cross_align(Alignment::Center)
			.padding(theme.gap3)
			.child(
				rect()
					.width(Size::fill())
					.height(Size::px(64.0))
					.center()
					.child("Migrate from Another Launcher")
					.font_size(18.0)
					.font_weight(FontWeight::BOLD),
			)
			.child(
				rect()
					.width(Size::fill())
					.height(Size::flex(1.0))
					.child(contents),
			)
			.child(
				rect()
					.width(Size::fill())
					.height(Size::px(64.0))
					.center()
					.child(
						icon_text_button("cycle", "Migrate", &theme)
							.width(Size::px(180.0))
							.active(&theme)
							.on_press(move |_| on_submit(())),
					),
			);

		rect()
			.expanded()
			.flex()
			.horizontal()
			.child(banner_image(SPLASH3, "splash3", true, &theme))
			.child(right)
	}
}

fn done_tab(theme: &Theme) -> impl IntoElement {
	let left = banner_image(SPLASH5, "splash5", true, theme);

	let padding = theme.gap3 * 2.5;

	let right = rect()
		.width(Size::flex(1.0))
		.height(Size::fill())
		.spacing(theme.gap3)
		.padding(padding)
		.cross_align(Alignment::Center)
		.font_size(20.0)
		.child(
			label()
				.font_size(24.0)
				.font_weight(FontWeight::BOLD)
				.text("Welcome to Nitrolaunch!"),
		)
		.child(
			label()
				.color(theme.fg2)
				.text("Now go create your first instance and start playing!"),
		);

	rect()
		.expanded()
		.flex()
		.horizontal()
		.child(left)
		.child(right)
}

fn banner_image(
	image: &'static [u8],
	asset_key: &'static str,
	is_left: bool,
	theme: &Theme,
) -> Rect {
	let radius = if is_left {
		CornerRadius::new(theme.round2, 0.0, 0.0, 0.0)
	} else {
		CornerRadius::new(0.0, theme.round2, 0.0, 0.0)
	};
	rect().width(Size::flex(1.5)).height(Size::fill()).child(
		ImageViewer::new((asset_key, image))
			.expanded()
			.opacity(0.75)
			.corner_radius(radius)
			.shiny_border(radius, &theme)
			.image_cover(ImageCover::Center)
			.aspect_ratio(AspectRatio::Max)
			.child(
				label()
					.text("BubbleFish")
					.position(Position::new_absolute().bottom(theme.gap2).left(theme.gap2))
					.color(theme.fg2)
					.font_size(16.0)
					.font_slant(FontSlant::Italic),
			),
	)
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
	Welcome,
	Plugins,
	Migrate,
	Accounts,
	Finished,
}

impl Tab {
	fn index(&self) -> usize {
		match self {
			Tab::Welcome => 0,
			Tab::Plugins => 1,
			Tab::Migrate => 2,
			Tab::Accounts => 3,
			Tab::Finished => 4,
		}
	}

	fn from_index(index: usize) -> Self {
		match index {
			0 => Tab::Welcome,
			1 => Tab::Plugins,
			2 => Tab::Migrate,
			3 => Tab::Accounts,
			4 => Tab::Finished,
			_ => panic!("Invalid tab index"),
		}
	}
}
