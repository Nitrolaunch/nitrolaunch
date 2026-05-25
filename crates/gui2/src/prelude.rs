pub use freya::prelude::*;
pub use freya::query::{
	Captured, Mutation, MutationCapability, Query, QueryCapability, QueryStateData, use_mutation,
	use_query,
};
pub use freya::radio::use_radio;

pub use crate::components::input::select::{InlineSelect, SelectOption};
pub use crate::components::{
	CustomEvents, CustomStyles, border_bottom, border_right, border_top, button, grid, icon_button,
};
pub use crate::icons::icon;
pub use crate::state::{BackState, FrontChannel, use_front_state};
pub use crate::theme::use_theme;
pub use crate::util::query_spawn;

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
