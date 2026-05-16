use crate::{
	components::dialog::modal::Modal, ops::instance::FetchInstanceOrTemplateConfig, prelude::*,
};
use nitrolaunch::config_crate::ConfigKind;

#[derive(PartialEq)]
pub struct ConfigPage;

impl Component for ConfigPage {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::ConfiguredItem);
		let item = front_state.read().configured_item().cloned();

		let title = match &item {
			Some(item) => match item.ty {
				ConfigKind::Instance => match &item.id {
					Some(id) => format!("Configuring instance {id}"),
					None => "Creating new instance".into(),
				},
				ConfigKind::Template => match &item.id {
					Some(id) => format!("Configuring template {id}"),
					None => "Creating new template".into(),
				},
				ConfigKind::BaseTemplate => "Configuring base template".into(),
			},
			None => "".into(),
		};

		Modal::new(title, "box".into())
			.maybe_child(item.is_some(), || ConfigModal {
				item: item.unwrap(),
			})
			.size_large()
			.on_close(move |_| front_state.write().set_configured_item(None))
	}
}

#[derive(PartialEq)]
struct ConfigModal {
	item: ConfiguredItem,
}

impl Component for ConfigModal {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let config_query = use_query(FetchInstanceOrTemplateConfig::new(
			self.item.clone(),
			back_state,
		));

		let config = config_query
			.read()
			.state()
			.ok()
			.cloned()
			.flatten()
			.unwrap_or_default();

		let config_str = serde_json::to_string(&config.main).unwrap();

		rect().center().child(config_str)
	}
}

/// Thing that is being configured
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConfiguredItem {
	/// The ID of what is being configured.
	///
	/// If it is empty, then either we are creating a new instance / template, or we are configuring the base template.
	pub id: Option<String>,
	pub ty: ConfigKind,
}
