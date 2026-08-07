use crate::prelude::*;

pub const MODAL_DEFAULT_WIDTH: f32 = 600.0;
pub const MODAL_DEFAULT_HEIGHT: f32 = 400.0;
pub const MODAL_MEDIUM_WIDTH: f32 = 800.0;
pub const MODAL_MEDIUM_HEIGHT: f32 = 600.0;
pub const MODAL_LARGE_WIDTH: f32 = 1000.0;
pub const MODAL_LARGE_HEIGHT: f32 = 750.0;
pub const MODAL_XLARGE_WIDTH: f32 = 1200.0;
pub const MODAL_XLARGE_HEIGHT: f32 = 750.0;

/// Base modal with no title or buttons
#[derive(PartialEq)]
pub struct ModalBase {
	child: Option<Element>,
	size: (f32, f32),
	on_close: EventHandler<()>,
}

impl ModalBase {
	pub fn new() -> Self {
		Self {
			child: None,
			size: (MODAL_DEFAULT_WIDTH, MODAL_DEFAULT_HEIGHT),
			on_close: (|_| {}).into(),
		}
	}

	pub fn maybe_child<E: IntoElement>(mut self, show: bool, f: impl FnOnce() -> E) -> Self {
		if show {
			self.child = Some(f().into_element());
		}
		self
	}

	pub fn size(mut self, width: f32, height: f32) -> Self {
		self.size = (width, height);
		self
	}

	pub fn on_close(mut self, handler: impl Into<EventHandler<()>>) -> Self {
		self.on_close = handler.into();
		self
	}
}

impl Component for ModalBase {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let on_close = self.on_close.clone();

		Popup::new()
			.width(Size::px(self.size.0))
			.color(theme.fg)
			.background(theme.panel)
			.padding(0.0)
			.maybe(self.child.is_some(), |this| {
				this.child(
					rect()
						.width(Size::fill())
						.height(Size::px(self.size.1))
						.border(theme.border(theme.panel_border))
						.corner_radius(theme.round2)
						.child(self.child.clone().unwrap()),
				)
			})
			.on_close_request(move |_| on_close.call(()))
	}
}

/// Popup modal with a titlebar and buttons
#[derive(PartialEq)]
pub struct Modal {
	child: Option<Element>,
	size: (f32, f32),
	on_close: EventHandler<()>,
	title: String,
	title_icon: String,
	hide_titlebar: bool,
	buttons: Vec<ModalButton>,
}

impl Modal {
	pub fn new(title: String, title_icon: String) -> Self {
		Self {
			child: None,
			size: (MODAL_DEFAULT_WIDTH, MODAL_DEFAULT_HEIGHT),
			on_close: (|_| {}).into(),
			title,
			title_icon,
			hide_titlebar: false,
			buttons: Vec::new(),
		}
	}

	pub fn new_no_title() -> Self {
		let mut out = Self::new(String::new(), String::new());
		out.hide_titlebar = true;
		out
	}

	pub fn maybe_child<E: IntoElement>(mut self, show: bool, f: impl FnOnce() -> E) -> Self {
		if show {
			self.child = Some(f().into_element());
		}
		self
	}

	pub fn size(mut self, width: f32, height: f32) -> Self {
		self.size = (width, height);
		self
	}

	pub fn size_large(self) -> Self {
		self.size(MODAL_LARGE_WIDTH, MODAL_LARGE_HEIGHT)
	}

	pub fn size_xlarge(self) -> Self {
		self.size(MODAL_XLARGE_WIDTH, MODAL_XLARGE_HEIGHT)
	}

	pub fn on_close(mut self, handler: impl Into<EventHandler<()>>) -> Self {
		self.on_close = handler.into();
		self
	}

	pub fn button(mut self, button: ModalButton) -> Self {
		self.buttons.push(button);
		self
	}

	pub fn buttons(mut self, buttons: impl IntoIterator<Item = ModalButton>) -> Self {
		self.buttons.extend(buttons);
		self
	}

	pub fn maybe_button(mut self, show: bool, button: ModalButton) -> Self {
		if show {
			self.buttons.push(button);
		}
		self
	}

	pub fn cancel_button(self) -> Self {
		let on_close = self.on_close.clone();
		self.button(ModalButton {
			title: "Cancel".into(),
			icon: "delete".into(),
			on_click: on_close,
			active: false,
		})
	}

	pub fn simple_confirm(
		title: &str,
		icon: &str,
		body: impl IntoElement,
		is_open: bool,
		on_close: impl Into<EventHandler<()>>,
		on_confirm: impl Into<EventHandler<()>>,
	) -> Self {
		Self::new(title.into(), icon.into())
			.on_close(on_close)
			.cancel_button()
			.button(ModalButton {
				title: "Confirm".into(),
				icon: "check".into(),
				on_click: on_confirm.into(),
				active: true,
			})
			.maybe_child(is_open, || body.into_element())
	}
}

impl Component for Modal {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let mut close_button_hovered = use_state(|| false);
		if self.child.is_none() {
			close_button_hovered.set(false);
		}

		let on_close = self.on_close.clone();

		ModalBase::new()
			.on_close(self.on_close.clone())
			.size(self.size.0, self.size.1)
			.maybe_child(self.child.is_some(), move || {
				let on_close = on_close.clone();

				let close_button_bg = if *close_button_hovered.read() {
					theme.item_hover
				} else {
					theme.panel
				};

				let close_button = rect()
					.width(Size::px(32.0))
					.height(Size::px(32.0))
					.center()
					.hover(close_button_hovered)
					.background(close_button_bg)
					.corner_radius(theme.round2)
					.on_press(move |_| on_close.call(()))
					.child(icon("delete", 16.0));

				let titlebar = rect()
					.width(Size::fill())
					.height(Size::px(40.0))
					.cont()
					.border(border_bottom(theme.border, theme.panel_border))
					.child(rect().width(Size::px(40.0)))
					.child(
						rect()
							.width(Size::flex(1.0))
							.height(Size::fill())
							.cont()
							.center()
							.font_weight(FontWeight::BOLD)
							.maybe(!self.title_icon.is_empty(), |this| {
								this.child(icon(&self.title_icon, 16.0))
							})
							.child(self.title.as_str()),
					)
					.child(
						rect()
							.width(Size::px(40.0))
							.height(Size::fill())
							.center()
							.child(close_button),
					);

				let buttons = self.buttons.iter().map(|x| {
					let on_click = x.on_click.clone();

					let (fg, bg, border) = (theme.primary, theme.primary_bg, theme.primary);

					icon_text_button(&x.icon, &x.title, &theme)
						.width(Size::px(180.0))
						.maybe(x.active, |this| {
							this.color(fg)
								.background(bg)
								.hover_background(bg)
								.border_fill(border)
						})
						.on_press(move |_| on_click.call(()))
						.into_element()
				});
				let bottom_bar = rect()
					.width(Size::fill())
					.padding(theme.gap)
					.horizontal()
					.main_align(Alignment::SpaceAround)
					.cross_align(Alignment::Center)
					.border(border_top(theme.border, theme.panel_border))
					.children(buttons);

				rect()
					.fill()
					.flex()
					.vertical()
					.maybe(!self.hide_titlebar, |this| this.child(titlebar))
					.child(
						rect()
							.width(Size::fill())
							.height(Size::flex(1.0))
							.child(self.child.clone().unwrap()),
					)
					.child(bottom_bar)
			})
	}
}

#[derive(PartialEq)]
pub struct ModalButton {
	pub title: String,
	pub icon: String,
	pub on_click: EventHandler<()>,
	pub active: bool,
}
