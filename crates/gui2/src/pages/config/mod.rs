use std::sync::Arc;

use crate::{
	components::{
		dialog::modal::{Modal, ModalButton},
		input::tabs::SideTabs,
	},
	ops::instance::{
		FetchInstanceOrTemplateConfig, FetchParentConfigs, SaveConfig, SaveConfigParams,
	},
	pages::config::{addons::AddonsConfig, general::GeneralTab},
	prelude::*,
	util::PtrEq,
};
use nitrolaunch::{
	config_crate::{
		ConfigKind,
		template::{TemplateConfig, TemplateLoaderConfiguration, TemplatePackageConfiguration},
	},
	core::util::versions::MinecraftVersion,
	instance::parse_loader_config,
	shared::{
		Side,
		loaders::Loader,
		util::{DeserListOrSingle, to_string_json},
		versions::{
			MinecraftLatestVersion, MinecraftVersionDeser, VersionPattern, format_versioned_string,
		},
	},
};

pub mod addons;
mod general;

#[derive(PartialEq)]
pub struct ConfigPage;

impl Component for ConfigPage {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::ConfiguredItem);
		let item = front_state.read().configured_item().cloned();
		let on_submit = use_state::<PtrEq<dyn Fn() -> bool>>(|| PtrEq(Arc::new(|| true)));
		let is_dirty = use_state(|| false);

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

		let front_state2 = front_state.clone();
		Modal::new(title, "box".into())
			.maybe_child(item.is_some(), || ConfigModal {
				item: item.unwrap(),
				on_submit: on_submit.clone(),
				is_dirty: is_dirty.clone(),
			})
			.size_large()
			.on_close(move |_| front_state.write().set_configured_item(None))
			.cancel_button()
			.button(ModalButton {
				title: "Save".into(),
				icon: "check".into(),
				on_click: EventHandler::from(move |_| {
					let successful = (on_submit.read().0)();
					if successful {
						front_state2.write().set_configured_item(None);
					}
				}),
				active: *is_dirty.read(),
			})
	}
}

#[derive(PartialEq)]
struct ConfigModal {
	item: ConfiguredItem,
	on_submit: State<PtrEq<dyn Fn() -> bool>>,
	is_dirty: State<bool>,
}

impl Component for ConfigModal {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let back_state = use_consume::<BackState>();
		let config_query = use_query(FetchInstanceOrTemplateConfig::new(
			self.item.clone(),
			back_state.clone(),
		));
		let save_config = use_mutation(Mutation::new(SaveConfig::new(back_state.clone()).toast(
			&back_state,
			Some("Saved"),
			"Failed to save config",
		)));

		let config_state = ConfigState::new(self.item.ty, self.item.is_new, self.is_dirty.clone());

		let parent_configs = use_query(FetchParentConfigs::new(
			config_state.from.read().cloned(),
			back_state.clone(),
		));
		let parent_configs = parent_configs
			.read()
			.state()
			.ok()
			.cloned()
			.unwrap_or_default();

		let id = self.item.id.clone();
		let item = self.item.clone();
		let mut config_state2 = config_state.clone();
		let mut on_submit_state = self.on_submit.clone();
		use_side_effect(move || {
			let config = config_query
				.read()
				.state()
				.ok()
				.cloned()
				.flatten()
				.unwrap_or_default();

			let original_config = config.no_templates.clone();
			config_state2.update(id.clone(), config.no_templates);

			// Set up on submit callback
			let config_state3 = config_state2.clone();
			let item = item.clone();
			let on_submit = move || {
				let Ok(config) = config_state3.apply(original_config.clone()) else {
					return false;
				};

				save_config.mutate(SaveConfigParams {
					item: item.clone(),
					config: NotEq(config),
				});

				true
			};
			on_submit_state.set(PtrEq(Arc::new(on_submit)));
		});

		let tab = use_state(|| Tab::General);
		let left_panel = rect()
			.width(Size::flex(1.0))
			.border(border_right(theme.border, theme.panel_border))
			.child(
				SideTabs::new(tab)
					.child(SelectOption::new(Tab::General, "General", Some("gear")))
					.child(SelectOption::new(
						Tab::Content,
						"Content",
						Some("honeycomb"),
					))
					.child(SelectOption::new(
						Tab::Launch,
						"Launch Settings",
						Some("play"),
					))
					.child(SelectOption::new(Tab::Plugins, "Plugins", Some("jigsaw"))),
			);

		let tab_contents = match &*tab.read() {
			Tab::General => GeneralTab {
				config_state,
				parent_configs: PtrEq(parent_configs.clone()),
			}
			.into_element(),
			Tab::Content => AddonsConfig {
				config_state,
				parent_configs: PtrEq(parent_configs.clone()),
			}
			.into_element(),
			Tab::Launch => rect().into_element(),
			Tab::Plugins => rect().into_element(),
		};

		let right_panel = rect().width(Size::flex(4.0)).child(tab_contents);

		rect()
			.horizontal()
			.flex()
			.child(left_panel)
			.child(right_panel)
	}
}

#[derive(PartialEq, Clone)]
enum Tab {
	General,
	Content,
	Launch,
	Plugins,
}

/// State objects for the config
#[derive(Clone, PartialEq)]
pub struct ConfigState {
	pub ty: ConfigKind,
	pub is_new: bool,
	/// Whether any of the config fields have been edited
	pub is_dirty: State<bool>,
	/// Whether we can propagate the name to the ID
	pub is_id_dirty: State<bool>,
	pub id: State<String>,
	pub from: State<Vec<String>>,
	pub name: State<Option<String>>,
	pub icon: State<Option<String>>,
	pub side: State<Option<Side>>,
	pub version: State<Option<String>>,
	pub client_loader: State<Option<Loader>>,
	pub server_loader: State<Option<Loader>>,
	pub client_loader_version: State<VersionPattern>,
	pub server_loader_version: State<VersionPattern>,
	pub packages: State<TemplatePackageConfiguration>,
	pub modpack: State<Option<String>>,
}

impl ConfigState {
	/// Must be called from component render scope
	pub fn new(ty: ConfigKind, is_new: bool, is_dirty: State<bool>) -> Self {
		let out = Self {
			ty,
			is_new,
			is_dirty,
			is_id_dirty: use_state(|| false),
			id: use_state(|| String::new()),
			from: use_state(|| Vec::new()),
			name: use_state(|| None),
			icon: use_state(|| None),
			side: use_state(|| None),
			version: use_state(|| None),
			client_loader: use_state(|| None),
			server_loader: use_state(|| None),
			client_loader_version: use_state(|| VersionPattern::Any),
			server_loader_version: use_state(|| VersionPattern::Any),
			packages: use_state(|| TemplatePackageConfiguration::default()),
			modpack: use_state(|| None),
		};

		use_side_effect(move || {
			out.id.read();
			out.from.read();
			out.name.read();
			out.icon.read();
			out.side.read();
			out.version.read();
			out.client_loader.read();
			out.server_loader.read();
			out.client_loader_version.read();
			out.server_loader_version.read();
			out.packages.read();
			out.modpack.read();

			out.is_dirty.clone().set(true);
		});

		out
	}

	pub fn update(&mut self, id: Option<String>, config: TemplateConfig) {
		if let Some(id) = id {
			self.id.set_if_modified(id);
		}

		self.from.set_if_modified(config.instance.from.get_vec());

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
		self.packages.set(match self.ty {
			ConfigKind::Instance => {
				TemplatePackageConfiguration::Simple(config.instance.packages.clone())
			}
			ConfigKind::Template | ConfigKind::BaseTemplate => config.packages.clone(),
		});
		self.modpack.set(config.instance.modpack);

		self.is_dirty.set_if_modified(self.is_new);
		self.is_id_dirty.set_if_modified(!self.is_new);
	}

	pub fn apply(&self, mut config: TemplateConfig) -> Result<TemplateConfig, ConfigError> {
		if self.id.peek().is_empty() {
			return Err(ConfigError::IdMissing);
		}

		config.instance.from = DeserListOrSingle::from_iter(self.from.peek().clone());
		config.instance.name = self.name.peek().clone();
		config.instance.icon = self.icon.peek().clone();
		config.instance.side = self.side.peek().clone();
		config.instance.version = match self.version.peek().as_deref() {
			None => None,
			Some("latest") => Some(MinecraftVersionDeser::Latest(
				MinecraftLatestVersion::Release,
			)),
			Some("latest_snapshot") => Some(MinecraftVersionDeser::Latest(
				MinecraftLatestVersion::Snapshot,
			)),
			Some(other) => Some(MinecraftVersionDeser::Version(other.into())),
		};
		config.instance.modpack = self.modpack.peek().clone();

		match self.ty {
			ConfigKind::Instance => {
				let Some(side) = self.side.peek().cloned() else {
					return Err(ConfigError::SideMissing);
				};
				config.instance.loader = match side {
					Side::Client => format_loader(
						self.client_loader.peek().as_ref(),
						&*self.client_loader_version.peek(),
					),
					Side::Server => format_loader(
						self.server_loader.peek().as_ref(),
						&*self.server_loader_version.peek(),
					),
				};
			}
			ConfigKind::Template | ConfigKind::BaseTemplate => {
				config.loader = TemplateLoaderConfiguration::default();
				let new_config = TemplateLoaderConfiguration::Full {
					client: format_loader(
						self.client_loader.peek().as_ref(),
						&*self.client_loader_version.peek(),
					),
					server: format_loader(
						self.server_loader.peek().as_ref(),
						&*self.server_loader_version.peek(),
					),
				};

				// This handles automatic simplification
				config.loader.merge(&new_config);
			}
		}

		match self.ty {
			ConfigKind::Instance => {
				config.instance.packages = self.packages.peek().iter_global().cloned().collect();
			}
			ConfigKind::Template | ConfigKind::BaseTemplate => {
				config.packages = self.packages.peek().cloned();
			}
		}

		Ok(config)
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

fn format_loader(loader: Option<&Loader>, version: &VersionPattern) -> Option<String> {
	if let Some(loader) = loader {
		Some(format_versioned_string(&to_string_json(loader), version))
	} else {
		None
	}
}

pub enum ConfigError {
	IdMissing,
	SideMissing,
}
