use std::{collections::HashMap, rc::Rc, sync::Arc};

use nitrolaunch::{
	plugin_crate::control::{Control, ControlSchema},
	shared::Side,
};
use serde_json::Value;

use crate::{
	components::input::{select::Selected, slider, switch::Switch, text::TextInput},
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

		let (value_str, is_str) = match &value {
			Value::String(s) => (s.clone(), true),
			_ => (String::new(), false),
		};
		let value_str = use_reactive(&value_str);
		let value_str2 = value_str;
		let on_set2 = on_set.clone();
		use_side_effect(move || {
			if !is_str {
				return;
			}

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
			ControlSchema::Number {
				min,
				max,
				step,
				slider: _,
			} => {
				if let (Some(min), Some(max)) = (min, max) {
					let min = *min as f64;
					let max = *max as f64;
					let step = *step as f64;
					let value = value.as_f64().unwrap_or(min);

					slider(
						value,
						min,
						max,
						step,
						move |value| {
							on_set.call(Value::Number(
								serde_json::Number::from_f64(value)
									.unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
							));
						},
						&theme,
						&front_state,
					)
					.into_element()
				} else {
					label().text("Not supported yet").into_element()
				}
			}
			ControlSchema::List { fields, is_map } => ObjectListControl {
				fields: fields.clone().into(),
				is_map: *is_map,
				value,
				on_set,
			}
			.into_element(),
			ControlSchema::CopyButton { text } => icon_text_button("copy", "Click to copy", &theme)
				.border_fill(theme.panel_border)
				.focus_border_fill(theme.panel_border)
				.on_press({
					let text = text.clone();
					move |_| {
						let _ = Clipboard::set(text.clone());
					}
				})
				.into_element(),
			_ => label()
				.text("Not supported yet")
				.color(theme.disabled)
				.into_element(),
		};

		let icon = self.control.icon.as_deref().unwrap_or("properties");

		field(&self.control.name, icon, &theme, control)
			.maybe(self.control.description.is_some(), |this| {
				this.tip(&front_state, self.control.description.as_deref().unwrap())
			})
			.into_element()
	}
}

#[derive(PartialEq)]
struct ObjectListControl {
	fields: Arc<[Control]>,
	is_map: bool,
	value: Value,
	on_set: EventHandler<Value>,
}

impl Component for ObjectListControl {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let mut values = match (&self.value, self.is_map) {
			(Value::Array(values), false) if values.iter().all(Value::is_object) => values
				.iter()
				.map(|value| (None, value.as_object().unwrap().clone()))
				.collect::<Vec<_>>(),
			(Value::Object(values), true) if values.values().all(Value::is_object) => values
				.iter()
				.map(|(key, value)| (Some(key.clone()), value.as_object().unwrap().clone()))
				.collect::<Vec<_>>(),
			_ => Vec::new(),
		};

		let on_set = self.on_set.clone();
		let is_map = self.is_map;
		let current_value = self.value.clone();
		let add = icon_text_button("plus", "Add", &theme)
			.border_fill(theme.panel_border)
			.focus_border_fill(theme.panel_border)
			.on_press(move |_| {
				let value = serde_json::Map::default();
				if is_map {
					let mut map = match &current_value {
						Value::Object(map) if map.values().all(Value::is_object) => map.clone(),
						_ => serde_json::Map::new(),
					};
					let mut key = "new".to_string();
					let mut suffix = 1;
					if let Value::Object(current) = &current_value {
						while current.contains_key(&key) {
							key = format!("new-{suffix}");
							suffix += 1;
						}
					}
					map.insert(key, Value::Object(value));
					on_set.call(Value::Object(map));
				} else {
					let mut values = match &current_value {
						Value::Array(values) if values.iter().all(Value::is_object) => {
							values.clone()
						}
						_ => Vec::new(),
					};
					values.push(Value::Object(value));
					on_set.call(Value::Array(values));
				}
			});

		let rows = values.drain(..).enumerate().map(|(index, (key, value))| {
			ObjectListRow {
				fields: self.fields.clone(),
				index,
				key,
				is_map: self.is_map,
				value: self.value.clone(),
				on_set: self.on_set.clone(),
				row: value,
			}
			.into_element()
		});

		rect()
			.width(Size::fill())
			.spacing(theme.gap2)
			.children(rows)
			.child(add)
	}
}

struct ObjectListRow {
	fields: Arc<[Control]>,
	index: usize,
	key: Option<String>,
	is_map: bool,
	value: Value,
	on_set: EventHandler<Value>,
	row: serde_json::Map<String, Value>,
}

impl PartialEq for ObjectListRow {
	fn eq(&self, other: &Self) -> bool {
		self.fields == other.fields
			&& self.index == other.index
			&& self.key == other.key
			&& self.is_map == other.is_map
			&& self.value == other.value
			&& self.row == other.row
	}
}

impl Component for ObjectListRow {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let is_map = self.is_map;
		let index = self.index;
		let fields = self.fields.clone();
		let original_value = self.value.clone();
		let current_key = self.key.clone();
		let row = self.row.clone();
		let row_config = ControlledConfig::new(row.clone());
		let row_on_set = self.on_set.clone();
		let child_controls = fields.iter().map(|control| {
			let id = control.id.clone();
			let mut child_row = row.clone();
			let child_on_set = row_on_set.clone();
			let child_original_value = original_value.clone();
			let child_key = current_key.clone();
			ControlInput {
				control: control.clone(),
				value: row_config.get(&id).cloned().unwrap_or(Value::Null),
				on_set: EventHandler::new(move |new| {
					set_value(&mut child_row, &id, new);
					child_on_set.call(replace_row(
						&child_original_value,
						is_map,
						index,
						child_key.as_deref(),
						child_row.clone(),
					));
				}),
			}
			.into_element()
		});
		let remove_value = original_value.clone();
		let remove_key = current_key.clone();
		let remove_on_set = self.on_set.clone();
		let remove = icon_button("trash", &theme).on_press(move |_| {
			if is_map {
				let mut values = remove_value.as_object().cloned().unwrap_or_default();
				if let Some(key) = remove_key.as_deref() {
					values.remove(key);
				}
				remove_on_set.call(Value::Object(values));
			} else {
				let values = remove_value.as_array().cloned().unwrap_or_default();
				remove_on_set.call(Value::Array(
					values
						.into_iter()
						.enumerate()
						.filter_map(|(row_index, value)| (row_index != index).then_some(value))
						.collect(),
				));
			}
		});
		let key_state = use_state(|| self.key.clone().unwrap_or_default());
		let key_on_set = self.on_set.clone();
		let value = self.value.clone();
		let old_key = self.key.clone();

		let key_input = TextInput::new(key_state).on_change(move |new_key: String| {
			if !is_map || Some(new_key.clone()) == old_key {
				return;
			}
			if let (Some(old_key), Value::Object(values)) = (&old_key, &value) {
				let mut values = values.clone();
				if let Some(value) = values.remove(old_key) {
					values.insert(new_key, value);
					key_on_set.call(Value::Object(values));
				}
			}
		});

		rect()
			.width(Size::fill())
			.padding(Gaps::new(theme.gap2, theme.gap2, 0.0, theme.gap2))
			.background(theme.panel)
			.border(theme.border(theme.panel_border))
			.corner_radius(theme.round)
			.spacing(theme.gap)
			.child(
				rect()
					.width(Size::fill())
					.cont()
					.cross_align(Alignment::Center)
					.maybe(self.is_map, |this| {
						this.child(
							rect()
								.horizontal()
								.cross_align(Alignment::Center)
								.color(theme.fg3)
								.child("Unique Key"),
						)
						.child(rect().width(Size::flex(1.0)).child(key_input))
					})
					.maybe(!self.is_map, |this| {
						this.child(rect().width(Size::flex(1.0)))
					})
					.child(remove),
			)
			.child(rect().children(child_controls))
	}
}

fn set_value(data: &mut serde_json::Map<String, Value>, key: &str, value: Value) {
	let mut config = ControlledConfig::new(std::mem::take(data));
	config.set(key, value);
	*data = config.into_data();
}

fn replace_row(
	value: &Value,
	is_map: bool,
	index: usize,
	key: Option<&str>,
	row: serde_json::Map<String, Value>,
) -> Value {
	if is_map {
		let mut values = value.as_object().cloned().unwrap_or_default();
		if let Some(key) = key {
			values.insert(key.to_string(), Value::Object(row));
		}
		Value::Object(values)
	} else {
		let mut values = value.as_array().cloned().unwrap_or_default();
		if let Some(value) = values.get_mut(index) {
			*value = Value::Object(row);
		}
		Value::Array(values)
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
				let section = sections.entry(section_id.clone()).or_default();
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
						let mut values = self.values;
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

#[derive(Default, Clone, Debug)]
pub struct ControlledConfig {
	data: serde_json::Map<String, Value>,
}

impl ControlledConfig {
	pub fn new(data: serde_json::Map<String, Value>) -> Self {
		Self { data }
	}

	pub fn update(&mut self, new_data: serde_json::Map<String, Value>) {
		self.data = new_data;
	}

	pub fn data(&self) -> &serde_json::Map<String, Value> {
		&self.data
	}

	pub fn into_data(self) -> serde_json::Map<String, Value> {
		self.data
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
		let mut value = Value::Object(self.data.clone());
		Self::optimize_value(&mut value);
		if let Value::Object(map) = value {
			self.data = map;
		}
	}

	fn optimize_value(value: &mut Value) {
		if let Value::Object(map) = value {
			map.retain(|_, v| {
				Self::optimize_value(v);
				!v.is_null() && !(v.is_object() && v.as_object().unwrap().is_empty())
			});
		}
	}
}

pub fn filter_control(control: &Control, side: Option<Side>) -> bool {
	if let ControlSchema::Section = &control.schema {
		return false;
	}
	if let Some(checked_side) = &control.side {
		side == Some(*checked_side)
	} else {
		true
	}
}
