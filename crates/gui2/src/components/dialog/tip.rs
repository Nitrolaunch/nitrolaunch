use std::rc::Rc;

use crate::{prelude::*, state::FrontState, util::Shared};

#[derive(PartialEq)]
pub struct Tips;

const OFFSET: f32 = 14.0;

impl Component for Tips {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Tip);
		let tip = front_state.read().tip().cloned().unwrap_or_default();

		// Nested layers instead of RelativeOverlay because it doesn't work
		rect()
			.position(
				Position::new_global()
					.top(tip.y + OFFSET)
					.left(tip.x + OFFSET),
			)
			.layer(Layer::Overlay)
			.child(
				rect()
					.padding(theme.gap2)
					.layer(Layer::Relative(100))
					.item_colorway(&theme, false, false)
					.border(theme.border(theme.secondary))
					.corner_radius(theme.round)
					.center()
					.cont()
					.maybe(front_state.read().tip().is_none(), |this| this.opacity(0.0))
					// Prevent quickly hovering the tip from freezing it in place
					.interactive(false)
					.child(icon("info", 16.0))
					.child(tip.tip.as_ref()),
			)
	}
}

#[derive(PartialEq, Clone, Default)]
pub struct Tip {
	pub x: f32,
	pub y: f32,
	pub tip: Rc<str>,
}

pub trait TipExt {
	fn tip(self, front_state: &Shared<FrontState>, tip: &str) -> Self;
}

impl<T: EventHandlersExt> TipExt for T {
	fn tip(self, front_state: &Shared<FrontState>, tip: &str) -> Self {
		let tip = tip.to_string();
		let front_state = front_state.clone();
		let front_state2 = front_state.clone();

		self.on_mouse_move(move |event: Event<MouseEventData>| {
			front_state.write().set_tip(Some(Tip {
				x: event.global_location.x as f32,
				y: event.global_location.y as f32,
				tip: tip.clone().into(),
			}));
		})
		.on_pointer_leave(move |_| {
			front_state2.write().set_tip(None);
		})
	}
}
