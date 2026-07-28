use crate::{
	components::{
		dialog::modal::{Modal, ModalButton},
		misc::socials,
	},
	prelude::*,
	state::use_launcher_data,
	util::assets::{LOGO_LARGE, SPLASH},
};

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
			Tab::Plugins => rect().into_element(),
			Tab::Migrate => rect().into_element(),
			Tab::Accounts => rect().into_element(),
			Tab::Finished => rect().into_element(),
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
	let mut radius = CornerRadius::new_all(0.0);
	radius.top_left = theme.round2;
	let left = rect().width(Size::flex(1.5)).height(Size::fill()).child(
		ImageViewer::new(("splash", SPLASH))
			.expanded()
			.opacity(0.75)
			.corner_radius(radius)
			.shiny_border(radius, &theme)
			.image_cover(ImageCover::Center)
			.aspect_ratio(AspectRatio::Max)
			.child(
				label()
					.text("BubbleFish")
					.position(
						Position::new_absolute()
							.bottom(theme.gap2)
							.left(theme.gap2),
					)
					.color(theme.fg2)
					.font_size(16.0)
					.font_slant(FontSlant::Italic),
			),
	);

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
					rect()
						.width(Size::fill())
						.padding(theme.gap3)
						.panel_colorway(theme, false, false)
						.corner_radius(theme.round)
						.child(
							label()
								.color(theme.fg3)
								.text(
									"Keep in mind the launcher is still in beta. There's lots of features on the horizon!",
								)
								.font_size(16.0),
						),
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
