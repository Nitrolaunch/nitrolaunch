use nitrolaunch::plugin_crate::hook::hooks::Popup;

use crate::{
	components::{
		dialog::modal::{Modal, ModalButton},
		input::control::{ControlledConfig, Controls},
	},
	ops::plugin_results::{RunCustomAction, RunCustomActionKeys},
	prelude::*,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct CustomPopupModal {
	pub popup: PtrEq<Popup>,
}

impl Component for CustomPopupModal {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let custom_action_mutation = use_mutation(Mutation::new(
			RunCustomAction::new(back_state.clone()).toast(
				&back_state,
				None,
				"Failed to run action",
			),
		));

		let control_state = use_state(ControlledConfig::default);

		let contents = Controls {
			controls: PtrEq(self.popup.0.controls.iter().cloned().collect()),
			values: control_state,
			side: None,
		};

		let front_state2 = front_state.clone();
		Modal::new(self.popup.0.title.clone(), self.popup.0.title_icon.clone())
			.maybe_child(true, || contents)
			.on_close(move |_| {
				front_state.write().set_modal(None);
			})
			.buttons(self.popup.0.buttons.iter().map(move |button| {
				let plugin = self.popup.0.plugin.clone();
				let button = button.clone();
				let custom_action_mutation = custom_action_mutation;
				let front_state2 = front_state2.clone();
				let control_state = control_state;

				ModalButton {
					title: button.title.clone(),
					icon: button.icon.clone(),
					on_click: EventHandler::from(move |_| {
						if let Some(action) = &button.action {
							let action = action.clone();
							let plugin = plugin.clone();
							let front_state2 = front_state2.clone();
							spawn(async move {
								let result = custom_action_mutation
									.mutate_async(RunCustomActionKeys {
										plugin: plugin.clone(),
										action: action.clone(),
										params: serde_json::Value::Null,
										related_id: None,
										control_state: control_state.read().data().clone(),
									})
									.await;
								if result.state().is_ok() {
									front_state2.write().set_modal(None);
								}
							});
						} else if button.closes {
							front_state2.write().set_modal(None);
						}
					}),
					active: button.active,
				}
			}))
	}
}
