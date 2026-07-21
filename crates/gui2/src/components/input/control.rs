use std::rc::Rc;

use nitrolaunch::plugin_crate::control::{Control, ControlSchema};
use serde_json::Value;

use crate::{
	components::input::{select::Selected, switch::Switch},
	prelude::*,
};

#[derive(PartialEq)]
pub struct ControlInput {
	control: Control,
	value: Value,
	on_set: EventHandler<Value>,
}

impl Component for ControlInput {
	fn render(&self) -> impl IntoElement {
		let value = self.value.clone();
		let on_set = self.on_set.clone();

		let control = match &self.control.schema {
			ControlSchema::Boolean => Switch {
				enabled: value == Value::Bool(true),
				on_toggle: EventHandler::new(move |_| {
					let new_value = match &value {
						Value::Bool(true) => Value::Bool(false),
						_ => Value::Bool(true),
					};
					on_set.call(new_value);
				}),
			}
			.into_element(),
			ControlSchema::Choice {
				variants,
				dropdown,
				multiple,
			} => {
				let null_value = "null".to_string();

				let selected = match value {
					Value::String(value) if !multiple => Selected::Single(value),
					Value::Array(values) if *multiple => Selected::Multi(
						values
							.iter()
							.filter_map(|x| x.as_str())
							.map(|x| x.to_string())
							.collect(),
					),
					Value::Null if !multiple => Selected::Single(null_value.clone()),
					Value::Null if *multiple => Selected::Multi(Vec::new()),
					_ if *multiple => Selected::Multi(Vec::new()),
					_ => return label().text("Value error").into_element(),
				};

				let multiple = *multiple;
				let on_select = Rc::new(move |selected: Selected<String>| {
					if multiple {
						on_set.call(Value::Array(
							selected.multi().into_iter().map(Value::String).collect(),
						));
					} else {
						on_set.call(Value::String(selected.single()));
					}
				});

				let children = variants.iter().map(|x| SelectOption {
					id: x.id.clone().unwrap_or_else(|| null_value.clone()),
					title: x.name.clone(),
					icon: None,
					tip: x.description.clone(),
					selected_colorway: None,
					action_button: None,
				});

				if *dropdown {
					Dropdown::new(selected, on_select)
						.children(children)
						.into_element()
				} else {
					InlineSelect::new(selected, on_select)
						.children(children)
						.into_element()
				}
			}
			_ => rect().into_element(),
		};

		control
	}
}
