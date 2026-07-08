pub use freya::prelude::*;
pub use freya::query::{
	Captured, Mutation, MutationCapability, Query, QueryCapability, QueryStateData, use_mutation,
	use_query,
};

pub use crate::components::dialog::tip::TipExt;
pub use crate::components::input::field;
pub use crate::components::input::select::{Dropdown, InlineSelect, SelectOption};
pub use crate::components::{
	CustomEvents, CustomStyles, FancyBorderExt, FancyBorderExtImage, border_bottom, border_left,
	border_right, border_top, button, clip_text, grid, icon_button, img, placeholder, segment,
	skeleton,
};
pub use crate::icons::icon;
pub use crate::ops::{ToastedMutationExt, ToastedQueryExt};
pub use crate::state::{BackState, FrontChannel, use_front_state};
pub use crate::theme::{Theme, use_theme};
pub use crate::util::{NotEq, query_spawn};

pub use crate::clone_into;

pub trait StateExt<T>: Clone {
	/// Returns an event handler that sets this state to a value
	fn setter(&self) -> EventHandler<T>;
}

impl<T: 'static> StateExt<T> for State<T> {
	fn setter(&self) -> EventHandler<T> {
		let mut this = self.clone();
		EventHandler::from(move |value| this.set(value))
	}
}

/// Transforms one state into a new state that modifies the value
pub fn use_transform<T: PartialEq + 'static, U: PartialEq + 'static>(
	mut state: State<T>,
	into: impl Fn(&T) -> U + 'static,
	back: impl Fn(&U) -> T + 'static,
) -> State<U> {
	let mut new_state = use_state(|| into(&*state.peek()));

	use_side_effect(move || {
		new_state.set_if_modified(into(&*state.read()));
	});
	use_side_effect(move || {
		state.set_if_modified(back(&*new_state.read()));
	});

	new_state
}

/// Transforms a state of Option<String> to a state of String
pub fn use_transform_optional_string(state: State<Option<String>>) -> State<String> {
	use_transform(
		state,
		|x| x.clone().unwrap_or_default(),
		|x| Some(x.clone()).filter(|x| !x.is_empty()),
	)
}
