use crate::{
	components::{dialog::modal::Modal, input::tabs::SideTabs},
	ops::instance::FetchInstanceOrTemplateConfig,
	pages::instance::config::general::GeneralTab,
	prelude::*,
};
use nitrolaunch::{
	config_crate::{ConfigKind, template::TemplateConfig},
	core::util::versions::MinecraftVersion,
	instance::parse_loader_config,
	shared::{Side, loaders::Loader, versions::VersionPattern},
};

mod general;

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
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let config_query = use_query(FetchInstanceOrTemplateConfig::new(
			self.item.clone(),
			back_state,
		));

		let config_state = ConfigState::new(self.item.ty, self.item.is_new);

		let id = self.item.id.clone();
		let mut config_state2 = config_state.clone();
		use_side_effect(move || {
			let config = config_query
				.read()
				.state()
				.ok()
				.cloned()
				.flatten()
				.unwrap_or_default();

			config_state2.update(id.clone(), config.editable);
		});

		let tab = use_state(|| Some("general".to_string()));
		let left_panel = rect()
			.width(Size::flex(1.0))
			.border(border_right(theme.border, theme.panel_border))
			.child(
				SideTabs::new(tab)
					.child(SelectOption::new("general", "General", Some("gear")))
					.child(SelectOption::new("packages", "Packages", Some("honeycomb")))
					.child(SelectOption::new("launch", "Launch Settings", Some("play")))
					.child(SelectOption::new("plugins", "Plugins", Some("jigsaw"))),
			);

		let tab_contents = match tab.read().as_deref() {
			None => rect().into_element(),
			Some("general") => GeneralTab { config_state }.into_element(),
			Some("packages") => rect().into_element(),
			Some("launch") => rect().into_element(),
			Some("plugins") => rect().into_element(),
			_ => rect().into_element(),
		};

		let right_panel = rect().width(Size::flex(4.0)).child(tab_contents);

		rect()
			.horizontal()
			.flex()
			.child(left_panel)
			.child(right_panel)
	}
}

/// State objects for the config
#[derive(Clone, PartialEq)]
struct ConfigState {
	ty: ConfigKind,
	is_new: bool,
	/// Whether any of the config fields have been edited
	is_dirty: State<bool>,
	/// Whether we can propagate the name to the ID
	is_id_dirty: State<bool>,
	id: State<String>,
	name: State<Option<String>>,
	icon: State<Option<String>>,
	side: State<Option<Side>>,
	version: State<Option<String>>,
	client_loader: State<Option<Loader>>,
	server_loader: State<Option<Loader>>,
	client_loader_version: State<VersionPattern>,
	server_loader_version: State<VersionPattern>,
}

impl ConfigState {
	/// Must be called from component render scope
	fn new(ty: ConfigKind, is_new: bool) -> Self {
		let out = Self {
			ty,
			is_new,
			is_dirty: use_state(|| false),
			is_id_dirty: use_state(|| false),
			id: use_state(|| String::new()),
			name: use_state(|| None),
			icon: use_state(|| None),
			side: use_state(|| None),
			version: use_state(|| None),
			client_loader: use_state(|| None),
			server_loader: use_state(|| None),
			client_loader_version: use_state(|| VersionPattern::Any),
			server_loader_version: use_state(|| VersionPattern::Any),
		};

		use_side_effect(move || {
			out.id.read();
			out.name.read();
			out.icon.read();
			out.side.read();
			out.version.read();
			out.client_loader.read();
			out.server_loader.read();
			out.client_loader_version.read();
			out.server_loader_version.read();

			out.is_dirty.clone().set(true);
		});

		out
	}

	fn update(&mut self, id: Option<String>, config: TemplateConfig) {
		self.is_id_dirty.set_if_modified(!self.is_new);
		if let Some(id) = id {
			self.id.set_if_modified(id);
		}

		let (loader, loader_version) = if let Some(loader) = config.client_loader() {
			let result = parse_loader_config(loader);
			(Some(result.0), result.1)
		} else {
			(None, VersionPattern::Any)
		};
		self.client_loader.set_if_modified(loader);
		self.client_loader_version.set_if_modified(loader_version);

		let (loader, loader_version) = if let Some(loader) = config.server_loader() {
			let result = parse_loader_config(loader);
			(Some(result.0), result.1)
		} else {
			(None, VersionPattern::Any)
		};
		self.server_loader.set_if_modified(loader);
		self.server_loader_version.set_if_modified(loader_version);

		self.name.set_if_modified(config.instance.name);
		self.icon.set_if_modified(config.instance.icon);
		self.side.set_if_modified(config.instance.side);
		self.version.set_if_modified(
			config
				.instance
				.version
				.map(|x| MinecraftVersion::from_deser(&x).to_string()),
		);
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
	/// Whether this is a new item
	pub is_new: bool,
}
