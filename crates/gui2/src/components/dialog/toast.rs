use std::time::Duration;

use freya::animation::{AnimNum, OnCreation, use_animation};

use crate::{components::TOAST_TIP_LAYER, prelude::*, state::BackEvent};

#[derive(PartialEq)]
pub struct Toasts;

impl Component for Toasts {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Toast);
		let toasts = front_state.read();
		let toasts = toasts.toasts();

		let front_state2 = front_state.clone();
		use_future(move || {
			let front_state2 = front_state2.clone();
			async move {
				let mut event_rx = front_state2.read().subscribe_events();
				while let Ok(event) = event_rx.recv().await {
					match event {
						BackEvent::SuccessToast(message) => {
							front_state2.write().toast(Toast::success(&message))
						}
						BackEvent::ErrorToast(title, contents) => front_state2
							.write()
							.toast(Toast::error(&title, contents.map(|x| x.into_element()))),
						_ => {}
					}
				}
			}
		});

		let toasts_width = 300.0;
		let toasts_height = 350.0;

		let toasts = toasts
			.into_iter()
			.map(|x| ToastElem { toast: x.clone() }.into_element());
		let toasts = ScrollView::new()
			.width(Size::fill())
			.height(Size::auto())
			.max_height(Size::px(toasts_height))
			.spacing(theme.gap)
			.children(toasts);
		let toasts = rect()
			.position(
				Position::new_absolute()
					.left(16.0 + theme.gap - toasts_width)
					.top(theme.input_height + theme.gap),
			)
			.width(Size::px(toasts_width))
			.layer(Layer::OverlayLevel(TOAST_TIP_LAYER))
			.child(toasts);

		rect()
			.padding(theme.gap)
			.corner_radius(theme.round)
			.border(theme.border(theme.item_border))
			.child(icon("notification", 16.0))
			.child(toasts)
	}
}

#[derive(PartialEq)]
struct ToastElem {
	toast: Toast,
}

impl Component for ToastElem {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let mut is_open = use_state(|| false);

		let lifetime = self.toast.ty.lifetime();
		let life = use_animation(move |config| {
			config.on_creation(OnCreation::Run);
			AnimNum::new(1.0, 0.0).duration(Duration::from_secs(lifetime as u64))
		});

		let life2 = life.clone();
		let id = self.toast.id;
		let front_state2 = front_state.clone();
		use_side_effect(move || {
			if !*life2.is_running().read() {
				front_state2.write().remove_toast(id);
			}
		});

		let (fg, bg) = match self.toast.ty {
			ToastType::Info => (theme.fg, theme.panel),
			ToastType::Success => (theme.success, theme.success_bg),
			ToastType::Warning => (theme.warning, theme.error_bg),
			ToastType::Error => (theme.error, theme.error_bg),
		};

		let front_state2 = front_state.clone();
		let header = rect()
			.width(Size::fill())
			.height(Size::px(32.0))
			.cont()
			.child(
				rect()
					.width(Size::px(32.0))
					.height(Size::fill())
					.center()
					.child(icon(self.toast.ty.icon(), 16.0)),
			)
			.child(
				rect()
					.width(Size::flex(1.0))
					.height(Size::fill())
					.center()
					.child(self.toast.title.clone()),
			)
			.child(
				rect()
					.width(Size::px(32.0))
					.height(Size::fill())
					.center()
					.on_press(move |_| front_state2.write().remove_toast(id))
					.child(icon("delete", 16.0)),
			);

		let progress = life.get().value() * 100.0;
		let progress = rect()
			.width(Size::fill())
			.padding(theme.gap)
			.center()
			.child(
				rect()
					.width(Size::percent(90.0))
					.height(Size::px(2.0))
					.child(
						rect()
							.width(Size::percent(progress))
							.height(Size::fill())
							.corner_radius(theme.round)
							.background(fg),
					),
			);

		rect()
			.width(Size::fill())
			.corner_radius(theme.round2)
			.color(fg)
			.border(theme.border(fg))
			.background(bg)
			.clickable()
			.layer(Layer::Relative(200))
			.on_press(move |_| is_open.toggle())
			.child(header)
			.maybe(*is_open.read() && self.toast.contents.is_some(), |this| {
				this.child(
					rect()
						.padding(theme.gap2)
						.child(self.toast.contents.clone().unwrap()),
				)
			})
			.child(progress)
	}

	fn render_key(&self) -> DiffKey {
		DiffKey::U64(self.toast.id as u64)
	}
}

#[derive(Clone, PartialEq)]
pub struct Toast {
	title: String,
	contents: Option<Element>,
	ty: ToastType,
	id: u32,
}

impl Toast {
	pub fn info(title: &str, contents: Option<Element>) -> Self {
		Self::new(title, contents, ToastType::Info)
	}

	pub fn success(title: &str) -> Self {
		Self::new(title, None, ToastType::Success)
	}

	pub fn warning(title: &str, contents: Option<Element>) -> Self {
		Self::new(title, contents, ToastType::Warning)
	}

	pub fn error(title: &str, contents: Option<Element>) -> Self {
		Self::new(title, contents, ToastType::Error)
	}

	pub fn new(title: &str, contents: Option<Element>, ty: ToastType) -> Self {
		Self {
			title: title.into(),
			contents,
			ty,
			id: 0,
		}
	}

	pub fn id(&self) -> u32 {
		self.id
	}

	pub fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
	Info,
	Success,
	Warning,
	Error,
}

impl ToastType {
	fn lifetime(&self) -> u8 {
		match self {
			Self::Info => 5,
			Self::Success => 3,
			Self::Warning | Self::Error => 8,
		}
	}

	fn icon(&self) -> &'static str {
		match self {
			Self::Info => "info",
			Self::Success => "check",
			Self::Warning => "warning",
			Self::Error => "error",
		}
	}
}
