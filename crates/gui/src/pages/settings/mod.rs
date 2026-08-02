use std::sync::Arc;

use nitrolaunch::{config::preferences::ConfigPreferences, shared::lang::Language};

use crate::{
	components::{
		console::{Console, ConsoleImpl},
		dialog::modal::{Modal, ModalButton},
		input::tabs::SideTabs,
	},
	data::LauncherData,
	ops::{
		misc::{FetchGlobalLog, FetchGlobalLogs, ShowDirectory, ShowDirectoryOption},
		settings::{FetchPreferences, SavePreferences},
	},
	pages::settings::{accounts::AccountSettings, general::GeneralSettings, plugins::PluginsPage},
	prelude::*,
	state::ModalType,
	util::PtrEq,
};

pub mod accounts;
mod general;
pub mod plugins;

#[derive(PartialEq)]
pub struct SettingsPage;

impl Component for SettingsPage {
	fn render(&self) -> impl IntoElement {
		let front_state = use_front_state();
		front_state.read().subscribe(FrontChannel::Modal);
		let tab = if let Some(ModalType::Settings(tab)) = front_state.read().modal() {
			Some(tab.clone())
		} else {
			None
		};
		let on_submit = use_state::<PtrEq<dyn Fn() -> bool>>(|| PtrEq(Arc::new(|| true)));
		let is_dirty = use_state(|| false);

		let front_state2 = front_state.clone();
		Modal::new("Settings".into(), "gear".into())
			.maybe_child(tab.is_some(), || SettingsModal {
				tab: tab.unwrap_or(Tab::General),
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
	tab: Tab,
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
		let show_directory = use_mutation(Mutation::new(ShowDirectory::new(back_state.clone())));

		let settings_state = SettingsState::new(self.is_dirty.clone());

		let front_state2 = front_state.clone();
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
			let front_state = front_state2.clone();
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

		let tab = use_state(|| self.tab.clone());
		let show_directory2 = show_directory.clone();
		let show_directory3 = show_directory.clone();

		let tabs = SideTabs::new(tab)
			.child(SelectOption::new(Tab::General, "General", Some("gear")))
			.child(SelectOption::new(
				Tab::Accounts,
				"Accounts",
				Some("multiple_users"),
			))
			.child(SelectOption::new(Tab::Plugins, "Plugins", Some("jigsaw")))
			.child(SelectOption::new(Tab::Logs, "Logs", Some("text")));

		#[cfg(debug_assertions)]
		let tabs = tabs.child(SelectOption::new(Tab::Debug, "Debug", Some("box")));

		let front_state2 = front_state.clone();
		let left_panel = rect()
			.width(Size::flex(1.0))
			.border(border_right(theme.border, theme.panel_border))
			.cont()
			.vertical()
			.child(
				rect()
					.width(Size::fill())
					.height(Size::flex(1.0))
					.child(tabs),
			)
			.child(
				rect()
					.width(Size::fill())
					.cont()
					.vertical()
					.padding(theme.gap2)
					.child(crate::components::input::tabs::Tab {
						option: SelectOption::new("", "Open Data Folder", Some("folder")),
						is_selected: false,
						on_select: EventHandler::from(move |_: &str| {
							show_directory2.mutate(ShowDirectoryOption::Data);
						}),
						horizontal: false,
					})
					.child(crate::components::input::tabs::Tab {
						option: SelectOption::new("", "Open Config Folder", Some("gear")),
						is_selected: false,
						on_select: EventHandler::from(move |_: &str| {
							show_directory3.mutate(ShowDirectoryOption::Config);
						}),
						horizontal: false,
					})
					.child(crate::components::input::tabs::Tab {
						option: SelectOption::new("", "Restart Onboarding", Some("refresh")),
						is_selected: false,
						on_select: EventHandler::from(move |_: &str| {
							front_state2.write().set_modal(Some(ModalType::Onboarding));
						}),
						horizontal: false,
					}),
			);

		let tab_contents = match &*tab.read() {
			Tab::General => GeneralSettings {
				state: settings_state.clone(),
			}
			.into_element(),
			Tab::Accounts => AccountSettings.into_element(),
			Tab::Plugins => PluginsPage.into_element(),
			Tab::Logs => SettingsConsole.into_element(),
			#[cfg(debug_assertions)]
			Tab::Debug => debug::DebugSettings.into_element(),
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
pub enum Tab {
	General,
	Accounts,
	Plugins,
	Logs,
	#[cfg(debug_assertions)]
	Debug,
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

#[derive(PartialEq)]
struct SettingsConsole;

impl Component for SettingsConsole {
	fn render(&self) -> impl IntoElement {
		let back_state = use_consume::<BackState>();
		let selected_log = use_state::<Option<String>>(|| None);
		let contents_query = use_query(Query::new(
			selected_log.read().clone(),
			FetchGlobalLog::new(back_state.clone()),
		));
		let logs = use_query(Query::new((), FetchGlobalLogs::new(back_state.clone())));

		let contents = contents_query.read().state().ok().cloned().map(PtrEq);

		let logs = logs.read().state().ok().cloned().unwrap_or_default();

		let console = Impl {
			contents,
			log_files: PtrEq(logs),
			selected_log,
			is_loading: !contents_query.read().state().is_ok(),
		};

		Console { console }
	}
}

#[derive(PartialEq, Clone)]
struct Impl {
	contents: Option<PtrEq<str>>,
	log_files: PtrEq<[String]>,
	selected_log: State<Option<String>>,
	is_loading: bool,
}

impl ConsoleImpl for Impl {
	fn contents(&self) -> Option<Arc<str>> {
		self.contents.as_ref().map(|x| x.0.clone())
	}

	fn is_loading(&self) -> bool {
		self.is_loading
	}

	fn get_log_files(&self) -> impl Iterator<Item = &String> {
		self.log_files.0.iter()
	}

	fn get_log_file(&self) -> Option<String> {
		self.selected_log.read().clone()
	}

	fn set_log_file(&self, file: Option<String>) {
		self.selected_log.clone().set(file);
	}
}

#[cfg(debug_assertions)]
mod debug {
	use nitrolaunch::shared::output::{MessageContents, NitroOutput};

	use crate::ops::task::Task;

	use super::*;

	#[derive(PartialEq)]
	pub struct DebugSettings;

	impl Component for DebugSettings {
		fn render(&self) -> impl IntoElement {
			let front_state = use_front_state();
			let back_state = use_consume::<BackState>();

			let front_state2 = front_state.clone();
			let front_state3 = front_state.clone();
			let front_state4 = front_state.clone();

			rect()
				.expanded()
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::WHITE)
						.on_press(move |_| {
							front_state.write().toast(Toast::info(
								"Info",
								Some("Lorem ipsum dolor sit amet adipiscing sdofijsdfoisjdfoisjdoflij".into_element()),
							));
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::GREEN)
						.on_press(move |_| {
							front_state2.write().toast(Toast::success("Success"));
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::YELLOW)
						.on_press(move |_| {
							front_state3.write().toast(Toast::warning(
								"Warning",
								Some("Lorem ipsum dolor sit amet adipiscing sdofijsdfoisjdfoisjdoflij".into_element()),
							));
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::RED)
						.on_press(move |_| {
							let contents =
								"Lorem ipsum dolor sit amet adipiscing sdofijsdfoisjdfoisjdoflij";
							front_state4.write().toast(
								Toast::error("Error", Some(contents.into_element()))
									.with_str_contents(contents.into()),
							);
						}),
				)
				.child(
					rect()
						.width(Size::px(16.0))
						.height(Size::px(16.0))
						.background(Color::BLUE)
						.on_press(move |_| {
							let back_state = back_state.clone();
							spawn_forever(async move {
								let mut o = back_state.output();
								o.set_task(Task::Opening);
								tokio::time::sleep(std::time::Duration::from_secs(5)).await;
								for i in 0..100 {
									o.display(MessageContents::associated(
										MessageContents::Simple("Downloading".into()),
										MessageContents::Progress {
											current: i as u32,
											total: 100,
										},
									));
									tokio::time::sleep(std::time::Duration::from_millis(100)).await;
								}
							});
						}),
				)
		}
	}
}
