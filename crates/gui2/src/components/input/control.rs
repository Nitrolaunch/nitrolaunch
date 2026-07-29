use std::{collections::HashMap, rc::Rc, sync::Arc};

use nitrolaunch::{
	plugin_crate::control::{Control, ControlSchema},
	shared::Side,
};
use serde_json::Value;

use crate::{
	components::input::{select::Selected, switch::Switch, text::TextInput},
	prelude::*,
	util::PtrEq,
};

#[derive(PartialEq)]
pub struct ControlInput {
	pub control: Control,
	pub value: Value,
	pub on_set: EventHandler<Value>,
}

impl Component for ControlInput {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();

		let value = self.value.clone();
		let on_set = self.on_set.clone();

		let value_str = match &value {
			Value::String(s) => s.clone(),
			_ => String::new(),
		};
		let value_str = use_reactive(&value_str);
		let value_str2 = value_str.clone();
		let on_set2 = on_set.clone();
		use_side_effect(move || {
			let value = value_str2.read().clone();
			let value = if value.is_empty() {
				Value::Null
			} else {
				Value::String(value)
			};
			on_set2.call(value);
		});

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
			ControlSchema::String { .. } => TextInput::new(value_str).into_element(),
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
						.panel_colorway()
						.children(children)
						.into_element()
				} else {
					InlineSelect::new(selected, on_select)
						.children(children)
						.into_element()
				}
			}
			_ => label()
				.text("Not supported yet")
				.color(theme.disabled)
				.into_element(),
		};

		let icon = self.control.icon.as_deref().unwrap_or_else(|| "properties");

		field(&self.control.name, icon, &theme, control)
			.maybe(self.control.description.is_some(), |this| {
				this.tip(&front_state, self.control.description.as_deref().unwrap())
			})
			.into_element()
	}
}

#[derive(Clone)]
pub struct ControlSection {
	pub id: String,
	pub name: String,
	pub icon: String,
	pub controls: Arc<[Control]>,
}

impl Default for ControlSection {
	fn default() -> Self {
		Self {
			id: String::new(),
			name: String::new(),
			icon: "box".into(),
			controls: Arc::default(),
		}
	}
}

impl ControlSection {
	pub fn sectionize(
		controls: &[Control],
		default_section: ControlSection,
	) -> HashMap<String, ControlSection> {
		let mut sections: HashMap<String, ControlSection> = HashMap::new();

		for control in controls {
			if let ControlSchema::Section = &control.schema {
				let section_id = control.id.clone();
				let section_name = control.name.clone();
				let section_icon = control.icon.clone().unwrap_or_else(|| "box".into());
				let section = sections
					.entry(section_id.clone())
					.or_insert_with(|| ControlSection::default());
				section.id = section_id;
				section.name = section_name;
				section.icon = section_icon;
			} else {
				let section = if let Some(section_id) = &control.section {
					sections
						.entry(section_id.clone())
						.or_insert_with(|| ControlSection {
							id: section_id.clone(),
							name: section_id.clone(),
							..Default::default()
						})
				} else {
					sections
						.entry(default_section.id.clone())
						.or_insert_with(|| default_section.clone())
				};
				section.controls = section
					.controls
					.iter()
					.cloned()
					.chain(std::iter::once(control.clone()))
					.collect();
			}
		}

		sections
	}
}

#[derive(PartialEq)]
pub struct Controls {
	pub controls: PtrEq<[Control]>,
	pub values: State<ControlledConfig>,
	pub side: Option<Side>,
}

impl Component for Controls {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		ScrollView::new().expanded().child(
			rect().padding(theme.gap3).children(
				self.controls
					.0
					.iter()
					.filter(|x| filter_control(x, self.side))
					.map(|x| {
						let id = x.id.clone();
						let mut values = self.values.clone();
						ControlInput {
							control: x.clone(),
							value: self
								.values
								.read()
								.get(&x.id)
								.cloned()
								.unwrap_or(Value::Null),
							on_set: EventHandler::new(move |new| {
								values.write().set(&id, new);
							}),
						}
						.into_element()
					}),
			),
		)
	}
}

#[derive(Default)]
pub struct ControlledConfig {
	data: serde_json::Map<String, Value>,
}

impl ControlledConfig {
	pub fn update(&mut self, new_data: serde_json::Map<String, Value>) {
		self.data = new_data;
	}

	pub fn data(&self) -> &serde_json::Map<String, Value> {
		&self.data
	}

	pub fn get(&self, key: &str) -> Option<&Value> {
		let components = key.split('.').collect::<Vec<_>>();
		let mut current = &self.data;
		for component in components.iter().take(components.len() - 1) {
			if let Some(Value::Object(map)) = current.get(*component) {
				current = map;
			} else {
				return None;
			}
		}
		current.get(*components.last().unwrap())
	}

	pub fn set(&mut self, key: &str, value: Value) {
		let components = key.split('.').collect::<Vec<_>>();
		let mut current = &mut self.data;
		for component in components.iter().take(components.len() - 1) {
			if !current.contains_key(*component) {
				current.insert(
					(*component).to_string(),
					Value::Object(serde_json::Map::new()),
				);
			}
			current = current
				.get_mut(*component)
				.unwrap()
				.as_object_mut()
				.unwrap();
		}
		current.insert(components.last().unwrap().to_string(), value);
	}

	pub fn optimize(&mut self) {
		Self::optimize_value(&mut Value::Object(self.data.clone()));
	}

	fn optimize_value(value: &mut Value) {
		match value {
			Value::Object(map) => {
				let keys_to_remove: Vec<String> = map
					.iter_mut()
					.filter_map(|(k, v)| {
						Self::optimize_value(v);
						if v.is_null() || (v.is_object() && v.as_object().unwrap().is_empty()) {
							None
						} else {
							Some(k.clone())
						}
					})
					.collect();
				for key in keys_to_remove {
					map.remove(&key);
				}
			}
			_ => {}
		}
	}
}

pub fn filter_control(control: &Control, side: Option<Side>) -> bool {
	if let Some(checked_side) = &control.side {
		side == Some(*checked_side)
	} else {
		true
	}
}
