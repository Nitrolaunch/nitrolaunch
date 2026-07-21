use std::sync::Arc;

use nitrolaunch::{config::preferences::ConfigPreferences, shared::lang::Language};

use crate::{
	components::{
		dialog::modal::{Modal, ModalButton},
		input::tabs::SideTabs,
	},
	data::LauncherData,
	ops::settings::{FetchPreferences, SavePreferences},
	pages::settings::{general::GeneralSettings, plugins::PluginsPage},
	prelude::*,
	state::ModalType,
	util::PtrEq,
};

mod general;
pub mod plugins;

#[derive(PartialEq)]
pub struct SettingsPage;

impl Component for SettingsPage {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Modal);
		let is_open = front_state.read().modal() == Some(&ModalType::Settings);
		let on_submit = use_state::<PtrEq<dyn Fn() -> bool>>(|| PtrEq(Arc::new(|| true)));
		let is_dirty = use_state(|| false);

		let front_state2 = front_state.clone();
		Modal::new("Settings".into(), "gear".into())
			.maybe_child(is_open, || SettingsModal {
				on_submit: on_submit.clone(),
				is_dirty: is_dirty.clone(),
			})
			.size_large()
			.on_close(move |_| front_state.write().set_modal(None))
			.cancel_button()
			.button(ModalButton {
				title: "Save".into(),
				icon: "check".into(),
				on_click: EventHandler::from(move |_| {
					let successful = (on_submit.read().0)();
					if successful {
						front_state2.write().set_modal(None);
					}
				}),
				active: *is_dirty.read(),
			})
	}
}

#[derive(PartialEq)]
struct SettingsModal {
	on_submit: State<PtrEq<dyn Fn() -> bool>>,
	is_dirty: State<bool>,
}

impl Component for SettingsModal {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();
		let front_state = use_front_state();
		let back_state = use_consume::<BackState>();
		let prefs_query = use_query(Query::new((), FetchPreferences::new(back_state.clone())));
		let data = use_hook(|| Arc::new(back_state.data()));
		let save_prefs = use_mutation(Mutation::new(
			SavePreferences::new(back_state.clone()).toast(
				&back_state,
				Some("Saved"),
				"Failed to save config",
			),
		));

		let settings_state = SettingsState::new(self.is_dirty.clone());

		let back_state2 = back_state.clone();
		let mut settings_state2 = settings_state.clone();
		let mut on_submit_state = self.on_submit.clone();
		use_side_effect(move || {
			let prefs = prefs_query.read().state().ok().cloned().unwrap_or_default();

			let original_prefs = prefs.clone();
			let original_data = data.clone();
			settings_state2.update(prefs, (*data).clone());

			// Set up on submit callback
			let settings_state3 = settings_state2.clone();
			let back_state2 = back_state2.clone();
			let front_state = front_state.clone();
			let on_submit = move || {
				let mut prefs = original_prefs.clone();
				let mut data = (*original_data).clone();
				let og_theme = data.base_theme.clone();
				let og_overlays = data.overlay_themes.clone();
				if settings_state3.apply(&mut prefs, &mut data).is_err() {
					return false;
				};

				if let Err(e) = data.write(&back_state2.paths) {
					front_state
						.write()
						.toast(Toast::from_error("Failed to write launcher data", e));
					return false;
				}
				save_prefs.mutate(NotEq(prefs));

				if og_theme != data.base_theme || og_overlays != data.overlay_themes {
					front_state.write().invalidate(FrontChannel::ThemeConfig);
				}

				front_state.write().invalidate(FrontChannel::Data);

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
					.child(SelectOption::new(Tab::Plugins, "Plugins", Some("jigsaw"))),
			);

		let tab_contents = match &*tab.read() {
			Tab::General => GeneralSettings {
				state: settings_state.clone(),
			}
			.into_element(),
			Tab::Plugins => PluginsPage.into_element(),
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
	Plugins,
}

/// State objects for the config
#[derive(Clone, PartialEq)]
struct SettingsState {
	is_dirty: State<bool>,
	language: State<Language>,
	base_theme: State<String>,
	overlay_themes: State<Vec<String>>,
}

impl SettingsState {
	/// Must be called from component render scope
	pub fn new(is_dirty: State<bool>) -> Self {
		let out = Self {
			is_dirty,
			language: use_state(|| Language::default()),
			base_theme: use_state(|| String::new()),
			overlay_themes: use_state(|| Vec::new()),
		};

		use_side_effect(move || {
			out.language.read();
			out.base_theme.read();
			out.overlay_themes.read();

			out.is_dirty.clone().set(true);
		});

		out
	}

	pub fn update(&mut self, prefs: ConfigPreferences, data: LauncherData) {
		self.language.set_if_modified(prefs.language);
		self.base_theme
			.set_if_modified(data.base_theme.unwrap_or("dark".into()));
		self.overlay_themes.set_if_modified(data.overlay_themes);

		self.is_dirty.set_if_modified(false);
	}

	pub fn apply(
		&self,
		prefs: &mut ConfigPreferences,
		data: &mut LauncherData,
	) -> Result<(), ConfigError> {
		prefs.language = *self.language.read();

		data.base_theme = Some(self.base_theme.read().clone());
		data.overlay_themes = self.overlay_themes.read().clone();

		Ok(())
	}
}

enum ConfigError {}
