use std::{
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::Duration,
};

use anyhow::Context;
use crossterm::event::{self, KeyCode, KeyEvent};
use image::DynamicImage;
use nitrolaunch::{
	config::Config,
	instance::update::manager::UpdateSettings,
	io::paths::Paths,
	pkg_crate::{metadata::PackageMetadata, PackageSearchResults, PkgRequest, PkgRequestSource},
	plugin_crate::hook::hooks::{
		AddCustomPackageRepositories, AddCustomPackageRepositoriesResult, AddSupportedLoaders,
	},
	shared::{
		loaders::Loader,
		output::NoOp,
		pkg::{ArcPkgReq, PackageSearchParameters},
		util::to_string_json,
		UpdateDepth,
	},
};
use ratatui::{
	layout::{Constraint, HorizontalAlignment, Layout, Margin, Rect},
	style::Style,
	widgets::{Block, Borders, Clear, List, ListState, Paragraph, Widget},
	DefaultTerminal, Frame,
};
use ratatui_image::{picker::Picker, Image, Resize};
use ratatui_textarea::TextArea;
use reqwest::Client;
use tokio::{
	sync::mpsc::{Receiver, Sender},
	task::JoinHandle,
};

use crate::{commands::CmdData, image_cache::ImageCache};

pub async fn run(mut data: CmdData<'_>) -> anyhow::Result<()> {
	data.ensure_config(true).await?;

	let repos = data
		.config
		.get_mut()
		.plugins
		.call_hook(AddCustomPackageRepositories, &(), &data.paths, &mut NoOp)
		.await?
		.flatten_all_results(&mut NoOp)
		.await?;

	let loaders = data
		.config
		.get_mut()
		.plugins
		.call_hook(AddSupportedLoaders, &(), &data.paths, &mut NoOp)
		.await?
		.flatten_all_results(&mut NoOp)
		.await?;

	let client = Client::new();
	let core = data
		.config
		.get()
		.get_core(
			None,
			&UpdateSettings {
				depth: UpdateDepth::Shallow,
				offline_auth: false,
			},
			&client,
			&data.config.get().plugins,
			&data.paths,
			&mut NoOp,
		)
		.await?;
	let versions = core
		.get_version_manifest(None, UpdateDepth::Shallow, &mut NoOp)
		.await?
		.get_releases();

	ratatui::run(move |terminal| {
		renderer(
			terminal,
			data.config.take().unwrap(),
			data.paths.clone(),
			repos,
			versions,
			loaders,
		)
	})
	.context("Failed to run app")
}

/// Main event loop
fn renderer(
	terminal: &mut DefaultTerminal,
	config: Config,
	paths: Paths,
	available_repos: Vec<AddCustomPackageRepositoriesResult>,
	available_versions: Vec<String>,
	available_loaders: Vec<Loader>,
) -> anyhow::Result<()> {
	let mut state = State::new(
		config,
		paths,
		available_repos,
		available_versions,
		available_loaders,
	)?;

	// Initial draw
	terminal.draw(|frame| render(frame, &mut state))?;

	loop {
		let mut should_render = false;

		// Check for results or state updates
		if let Ok(update) = state.worker_state_rx.try_recv() {
			state.worker_state = update;
			should_render = true;
		}

		if let Ok(results) = state.results_rx.try_recv() {
			state.results = results;
			state.package_list_state.select_first();
			state.select_package();
			should_render = true;
		}

		if let Ok(info) = state.package_info_rx.try_recv() {
			state.package_info = Some(info);
			state.preview_scroll = 0;
			should_render = true;
		}

		if state.image_available.swap(false, Ordering::Relaxed) {
			should_render = true;
		}

		// Key checks. If there is no key event, no need to re-render
		let key = get_key()?;
		if let Some(key) = key {
			should_render = true;

			match state.focus {
				_ if key.code == KeyCode::Esc => state.focus_none(),
				_ if key.code == KeyCode::Char('q') && state.focus != FocusState::Search => break,
				FocusState::None => match key.code {
					KeyCode::Char('s') | KeyCode::Char('/') => state.focus = FocusState::Search,
					KeyCode::Char('x') => state.reset_filters(),
					KeyCode::Char('r') => state.focus_popup(Popup::Repository),
					KeyCode::Char('t') => state.focus_popup(Popup::PackageType),
					KeyCode::Char('v') => state.focus_popup(Popup::Version),
					KeyCode::Char('l') => state.focus_popup(Popup::Loader),
					KeyCode::Char('c') => state.focus_popup(Popup::Category),
					KeyCode::Char('p') | KeyCode::Tab => state.focus = FocusState::Preview,
					KeyCode::Up | KeyCode::Char('k') => {
						state.package_list_state.select_previous();
						state.preview_package();
					}
					KeyCode::Down | KeyCode::Char('j') => {
						state.package_list_state.select_next();
						state.preview_package();
					}
					KeyCode::Enter => {
						state.select_package();
						state.focus = FocusState::Preview;
					}
					_ => {}
				},
				FocusState::Search => match key.code {
					KeyCode::Enter => {
						if let Some(line) = state.search.lines().first() {
							if line.is_empty() {
								state.search_params.inner.search = None;
							} else {
								state.search_params.inner.search = Some(line.clone());
							}
						}
						state.search();
						state.focus_none();
					}
					_ => {
						state.search.input(key);
					}
				},
				FocusState::Popup(popup) => popup.input(&mut state, key),
				FocusState::Preview => match key.code {
					KeyCode::Char('p') | KeyCode::Tab => state.focus_none(),
					KeyCode::Char('d') => state.focus_preview_tab(PreviewTab::Description),
					KeyCode::Char('v') => state.focus_preview_tab(PreviewTab::Versions),
					KeyCode::Char('g') => state.focus_preview_tab(PreviewTab::Gallery),
					KeyCode::Up | KeyCode::Char('k') if state.preview_scroll > 0 => {
						state.preview_scroll -= 1
					}
					KeyCode::Down | KeyCode::Char('j')
						if state.preview_scroll < state.preview_scroll_height =>
					{
						state.preview_scroll += 1
					}
					_ => {}
				},
			}
		};

		// Re-render
		if should_render {
			terminal.draw(|frame| render(frame, &mut state))?;
		}
	}
	Ok(())
}

/// Main render
fn render(frame: &mut Frame, state: &mut State) {
	let layout = Layout::vertical([
		Constraint::Length(1),
		Constraint::Fill(1),
		Constraint::Length(3),
		Constraint::Length(1),
	]);
	let layout = frame.area().layout::<4>(&layout);
	let filters_pane = layout[0];
	let preview_pane = layout[1];
	let search_pane = layout[2];
	let status_pane = layout[3];

	// Draw panes

	// Status bar
	let status_layout = Layout::horizontal([Constraint::Fill(3), Constraint::Fill(1)]);
	let status_layout = status_pane.layout::<2>(&status_layout);
	let keybinds_pane = status_layout[0];
	let state_pane = status_layout[1];

	let keybinds_text = match state.focus {
		FocusState::None => {
			"q to quit; s to search; k/j for up/down; enter to select; tab/p to focus package"
		}
		FocusState::Search => "esc to exit search; enter to submit",
		FocusState::Popup(..) => "esc to exit popup; k/j for up/down; enter to select",
		FocusState::Preview => "esc to exit preview; k/j for scroll",
	};

	let keybinds = Paragraph::new(keybinds_text);
	frame.render_widget(keybinds, keybinds_pane);

	let worker_state = match &state.worker_state {
		WorkerState::Idle => Paragraph::new("Idle").style(Style::new().gray()),
		WorkerState::Running => Paragraph::new("Running"),
		WorkerState::Success => Paragraph::new("Success").style(Style::new().light_green()),
		WorkerState::Error(error) => Paragraph::new(error.as_str()).style(Style::new().red()),
	};
	frame.render_widget(worker_state, state_pane);

	// Search bar
	let search_block = state.search.block().unwrap();
	let search_block_style = if state.focus == FocusState::Search {
		Style::new().green()
	} else {
		Style::new().gray()
	};
	let search_block = search_block.clone().border_style(search_block_style);
	state.search.set_block(search_block);
	frame.render_widget(&state.search, search_pane);

	// Package preview
	let preview_layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(4)]);
	let preview_layout = preview_pane.layout::<2>(&preview_layout);
	let list_pane = preview_layout[0];
	let preview_pane = preview_layout[1];

	// Package list
	let package_items = state.results.results.iter().map(|x| {
		if let Some(preview) = state.results.previews.get(x) {
			if let Some(name) = &preview.0.name {
				name.clone()
			} else {
				x.clone()
			}
		} else {
			x.clone()
		}
	});
	state.package_list = state.package_list.clone().items(package_items);
	let mut block = Block::bordered().title("Packages");
	if state.focus == FocusState::None {
		block = block.border_style(Style::new().green());
	}
	let inner = block.inner(list_pane);
	frame.render_widget(block, list_pane);
	frame.render_stateful_widget(&state.package_list, inner, &mut state.package_list_state);

	// Preview pane
	let mut block = Block::bordered().title("Preview");
	if state.focus == FocusState::Preview {
		block = block.border_style(Style::new().green());
	}
	let inner_area = block.inner(preview_pane);
	frame.render_widget(block, preview_pane);
	if let Some(req) = state.get_selected_package() {
		let mut scroll_height = 0;
		let widget = PackageInfoWidget {
			req,
			info: state.package_info.as_ref(),
			state: state,
			scroll_height: &mut scroll_height,
		};

		frame.render_widget(widget, inner_area);
		state.preview_scroll_height = scroll_height;
	} else {
		frame.render_widget(Clear, inner_area);
	};

	// Filters
	let filter_layout = Layout::horizontal(Constraint::from_fills([1, 1, 1, 1, 1]));
	let filter_layout = filters_pane.layout::<5>(&filter_layout);
	let repo_pane = filter_layout[0];
	let type_pane = filter_layout[1];
	let version_pane = filter_layout[2];
	let loader_pane = filter_layout[3];
	let category_pane = filter_layout[4];

	let repo = format!(
		"[r] Repository: {}",
		state.search_params.repo.as_deref().unwrap_or("Any")
	);
	let repo = Paragraph::new(repo).style(Style::new().bold().light_blue());
	frame.render_widget(repo, repo_pane);

	let ty = format!(
		"[t] Package type: {}",
		state
			.search_params
			.inner
			.types
			.first()
			.map(|x| x.to_string())
			.unwrap_or("Any".into())
	);
	let ty = Paragraph::new(ty).style(Style::new().bold().light_blue());
	frame.render_widget(ty, type_pane);

	let version = match state.search_params.inner.minecraft_versions.len() {
		0 => "Any",
		1 => state
			.search_params
			.inner
			.minecraft_versions
			.first()
			.unwrap()
			.as_str(),
		_ => "Multiple",
	};
	let version =
		Paragraph::new(format!("[v] Version: {version}")).style(Style::new().bold().light_blue());
	frame.render_widget(version, version_pane);

	let loader = match state.search_params.inner.loaders.len() {
		0 => "Any".to_string(),
		1 => state
			.search_params
			.inner
			.loaders
			.first()
			.unwrap()
			.to_string(),
		_ => "Multiple".to_string(),
	};
	let loader =
		Paragraph::new(format!("[l] Loader: {loader}")).style(Style::new().bold().light_blue());
	frame.render_widget(loader, loader_pane);

	let category = match state.search_params.inner.categories.len() {
		0 => "Any".to_string(),
		1 => to_string_json(state.search_params.inner.categories.first().unwrap()),
		_ => "Multiple".to_string(),
	};
	let category =
		Paragraph::new(format!("[c] Category: {category}")).style(Style::new().bold().light_blue());
	frame.render_widget(category, category_pane);

	// Popup
	if let FocusState::Popup(popup) = state.focus {
		let mut base_area = frame.area().inner(Margin::new(1, 1));
		base_area.height = 10;

		popup.render(state, frame, base_area);
	}
}

/// State for the application
struct State<'a> {
	/// Handle for worker thread
	worker: JoinHandle<()>,
	/// Current state of the worker thread
	worker_state: WorkerState,
	/// Receiver for worker state updates
	worker_state_rx: Receiver<WorkerState>,
	/// Sender for worker thread tasks
	task_tx: Sender<Task>,
	/// Receiver for search results
	results_rx: Receiver<PackageSearchResults>,
	/// Finalized search results
	results: PackageSearchResults,
	/// Receiver for package info
	package_info_rx: Receiver<PackageInfo>,
	/// Finalized package info
	package_info: Option<PackageInfo>,
	/// Image cache
	image_cache: ImageCache,
	/// Set to true when a new image was loaded and we should re-render
	image_available: Arc<AtomicBool>,
	/// Current focus state
	focus: FocusState,
	/// Search parameters
	search_params: SearchParams,
	/// Search bar
	search: TextArea<'a>,
	/// Popup list state
	popup_list_state: ListState,
	/// List of packages
	package_list: List<'a>,
	/// List state for package list
	package_list_state: ListState,
	/// Last selected package
	last_selected_package: Option<ArcPkgReq>,
	/// Available repositories
	repositories: Vec<AddCustomPackageRepositoriesResult>,
	/// Available Minecraft versions
	versions: Vec<String>,
	/// Available loaders
	loaders: Vec<Loader>,
	/// Current scroll of preview pane body
	preview_scroll: u16,
	/// Max height scroll of preview pane body
	preview_scroll_height: u16,
	/// Current tab of preview pane
	preview_tab: PreviewTab,
}

impl<'a> State<'a> {
	/// Initialize state with widgets and worker thread
	fn new(
		config: Config,
		paths: Paths,
		repositories: Vec<AddCustomPackageRepositoriesResult>,
		versions: Vec<String>,
		loaders: Vec<Loader>,
	) -> anyhow::Result<Self> {
		// Get info

		// Setup worker
		let (state_tx, state_rx) = tokio::sync::mpsc::channel(2);
		let (task_tx, task_rx) = tokio::sync::mpsc::channel(2);
		let (results_tx, results_rx) = tokio::sync::mpsc::channel(2);
		let (package_info_tx, package_info_rx) = tokio::sync::mpsc::channel(2);
		let handle = tokio::spawn(worker_thread(
			config,
			paths,
			state_tx,
			task_rx,
			results_tx,
			package_info_tx,
		));

		// Search bar
		let mut search = TextArea::new(Vec::new());
		search.set_style(Style::new().white());
		search.set_placeholder_text("Enter search...");
		let search_block = Block::bordered().title("Search");
		search.set_block(search_block);

		// Package list
		let package_list = List::default()
			.highlight_style(Style::new().green())
			.highlight_symbol(">");
		let mut package_list_state = ListState::default();
		package_list_state.select_first();

		Ok(Self {
			worker: handle,
			worker_state: WorkerState::Idle,
			worker_state_rx: state_rx,
			task_tx,
			results_rx,
			package_info_rx,
			package_info: None,
			image_cache: ImageCache::new(Client::new()),
			image_available: Arc::new(AtomicBool::new(false)),
			results: PackageSearchResults::default(),
			search_params: SearchParams {
				inner: PackageSearchParameters {
					count: 35,
					skip: 0,
					..Default::default()
				},
				repo: None,
			},
			popup_list_state: ListState::default(),
			search,
			package_list,
			package_list_state,
			last_selected_package: None,
			repositories,
			versions,
			loaders,
			focus: FocusState::None,
			preview_scroll: 0,
			preview_scroll_height: 0,
			preview_tab: PreviewTab::Description,
		})
	}

	/// Gets the currently selected package
	fn get_selected_package(&self) -> Option<ArcPkgReq> {
		let pos = self.package_list_state.selected()?;
		let pkg = self.results.results.get(pos)?;

		Some(PkgRequest::parse(pkg, PkgRequestSource::UserRequire).arc())
	}

	/// Gets info for the currently selected repository
	fn get_selected_repo_info(&self) -> Option<&AddCustomPackageRepositoriesResult> {
		self.repositories
			.iter()
			.find(|x| Some(&x.id) == self.search_params.repo.as_ref())
	}

	/// Returns focus to the default
	fn focus_none(&mut self) {
		// Search when a multi-select closes
		if let FocusState::Popup(Popup::Version | Popup::Loader | Popup::Category) = self.focus {
			self.search();
		}
		self.focus = FocusState::None;
	}

	/// Focuses a popup
	fn focus_popup(&mut self, popup: Popup) {
		self.popup_list_state.select_first();
		self.focus = FocusState::Popup(popup);
	}

	/// Focuses a different preview tab
	fn focus_preview_tab(&mut self, tab: PreviewTab) {
		self.preview_tab = tab;
		self.preview_scroll = 0;
	}

	/// Resets package filters
	fn reset_filters(&mut self) {
		self.search_params.inner.types = Vec::new();
		self.search_params.inner.minecraft_versions = Vec::new();
		self.search_params.inner.loaders = Vec::new();
		self.search_params.inner.categories = Vec::new();
		self.search();
	}

	/// Sends a request to search for packages given the current parameters
	fn search(&mut self) {
		let _ = self
			.task_tx
			.try_send(Task::FetchPackages(self.search_params.clone()));
	}

	/// Previews the currently highlighted package in the list
	fn preview_package(&mut self) {
		let Some(req) = self.get_selected_package() else {
			return;
		};

		if let Some(preview) = self.results.previews.get(&req.to_string()) {
			self.package_info = Some(PackageInfo {
				meta: Arc::new(preview.0.clone()),
			});
			self.preview_scroll = 0;
		}
	}

	/// Selects the currently highlighted package in the list and sends a request to fetch it
	fn select_package(&mut self) {
		let Some(req) = self.get_selected_package() else {
			return;
		};

		self.last_selected_package = Some(req.clone());
		let _ = self.task_tx.try_send(Task::FetchPackageInfo(req));
	}

	/// Requests an image to be downloaded
	fn request_image(&self, url: &str) {
		let url = url.to_string();
		let cache = self.image_cache.clone();
		let signal = self.image_available.clone();

		tokio::spawn(async move {
			let _ = cache.get(&url).await;
			signal.store(true, Ordering::Relaxed);
		});
	}
}

impl<'a> Drop for State<'a> {
	fn drop(&mut self) {
		self.worker.abort();
	}
}

/// Loop for working tokio task that does stuff like fetch search results
async fn worker_thread(
	config: Config,
	paths: Paths,
	state_tx: Sender<WorkerState>,
	mut task_rx: Receiver<Task>,
	results_tx: Sender<PackageSearchResults>,
	package_info_tx: Sender<PackageInfo>,
) {
	let client = Client::new();

	loop {
		let Some(task) = task_rx.recv().await else {
			continue;
		};

		match task {
			Task::FetchPackages(params) => {
				let _ = state_tx.try_send(WorkerState::Running);
				let results = config
					.packages
					.search(
						params.inner,
						params.repo.as_deref(),
						&paths,
						&client,
						&mut NoOp,
					)
					.await;

				match results {
					Ok(results) => {
						let _ = results_tx.send(results).await;
						let _ = state_tx.try_send(WorkerState::Success);
					}
					Err(e) => {
						let _ = state_tx.try_send(WorkerState::Error(e.to_string()));
					}
				}
			}
			Task::FetchPackageInfo(req) => {
				let _ = state_tx.try_send(WorkerState::Running);
				let pkg = config.packages.get(&req, &paths, &client, &mut NoOp).await;
				let pkg = match pkg {
					Ok(pkg) => pkg,
					Err(e) => {
						let _ = state_tx.try_send(WorkerState::Error(e.to_string()));
						continue;
					}
				};

				let Ok(meta) = pkg.get_metadata(&paths, &client).await else {
					let _ = state_tx.try_send(WorkerState::Error("Failed to get metadata".into()));
					continue;
				};

				let _ = package_info_tx.send(PackageInfo { meta }).await;
				let _ = state_tx.try_send(WorkerState::Success);
			}
		}
	}
}

/// Task that the worker thread can run
enum Task {
	FetchPackages(SearchParams),
	FetchPackageInfo(ArcPkgReq),
}

/// State of the worker thread
enum WorkerState {
	Idle,
	Running,
	Success,
	Error(String),
}

/// Focus state for the TUI
#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusState {
	None,
	Search,
	Popup(Popup),
	Preview,
}

/// Different selection popups
#[derive(Clone, Copy, PartialEq, Eq)]
enum Popup {
	Repository,
	PackageType,
	Version,
	Loader,
	Category,
}

impl Popup {
	fn render(&self, state: &mut State, frame: &mut Frame, area: Rect) {
		let layout = Layout::horizontal(Constraint::from_fills([1, 1, 1, 1, 1]));
		let layout = area.layout::<5>(&layout);

		let area = match self {
			Self::Repository => layout[0],
			Self::PackageType => layout[1],
			Self::Version => layout[2],
			Self::Loader => layout[3],
			Self::Category => layout[4],
		};

		let block = Block::bordered()
			.border_style(Style::new().green())
			.title(self.title());

		let selected = self.get_selected(state);
		let items = self.get_items(state);
		let items = items.into_iter().enumerate().map(|(i, x)| {
			if selected.contains(&i) {
				format!("{x} (Selected)")
			} else {
				x
			}
		});

		let list = List::default()
			.block(block)
			.items(items)
			.highlight_symbol(">")
			.highlight_style(Style::new().green());

		frame.render_widget(Clear, area);
		frame.render_stateful_widget(list, area, &mut state.popup_list_state);
	}

	fn input(&self, state: &mut State, input: KeyEvent) {
		match input.code {
			KeyCode::Char('r') if *self == Popup::Repository => state.focus_none(),
			KeyCode::Char('t') if *self == Popup::PackageType => state.focus_none(),
			KeyCode::Char('v') if *self == Popup::Version => state.focus_none(),
			KeyCode::Char('l') if *self == Popup::Loader => state.focus_none(),
			KeyCode::Char('c') if *self == Popup::Category => state.focus_none(),
			KeyCode::Up | KeyCode::Char('k') => state.popup_list_state.select_previous(),
			KeyCode::Down | KeyCode::Char('j') => state.popup_list_state.select_next(),
			KeyCode::Enter => match self {
				// Single select
				Self::Repository | Self::PackageType => {
					if let Some(selected) = state.popup_list_state.selected() {
						self.select(state, selected);
						state.search();
						state.focus = FocusState::None;
					}
				}
				// Multi select
				Self::Version | Self::Loader | Self::Category => {
					if let Some(selected) = state.popup_list_state.selected() {
						self.select(state, selected);
						state.search();
					}
				}
			},
			_ => {}
		}
	}

	fn select(&self, state: &mut State, pos: usize) {
		match self {
			Self::Repository => {
				let Some(repo) = state.repositories.get(pos) else {
					return;
				};

				state.search_params.repo = Some(repo.id.clone());
			}
			Self::PackageType => {
				let Some(repo) = state.get_selected_repo_info() else {
					return;
				};
				let Some(ty) = repo.metadata.package_types.get(pos) else {
					return;
				};

				state.search_params.inner.types = vec![ty.clone()];
			}
			Self::Version => {
				let Some(version) = state.versions.get(pos) else {
					return;
				};

				if state
					.search_params
					.inner
					.minecraft_versions
					.contains(version)
				{
					state
						.search_params
						.inner
						.minecraft_versions
						.retain(|x| x != version);
				} else {
					state
						.search_params
						.inner
						.minecraft_versions
						.push(version.clone());
				}
			}
			Self::Loader => {
				let Some(loader) = state.loaders.get(pos) else {
					return;
				};

				if state.search_params.inner.loaders.contains(loader) {
					state.search_params.inner.loaders.retain(|x| x != loader);
				} else {
					state.search_params.inner.loaders.push(loader.clone());
				}
			}
			Self::Category => {
				let Some(repo) = state.get_selected_repo_info() else {
					return;
				};
				let Some(category) = repo.metadata.package_categories.get(pos) else {
					return;
				};
				let category = category.clone();

				if state.search_params.inner.categories.contains(&category) {
					state
						.search_params
						.inner
						.categories
						.retain(|x| x != &category);
				} else {
					state.search_params.inner.categories.push(category.clone());
				}
			}
		}
	}

	fn get_selected(&self, state: &State) -> Vec<usize> {
		match self {
			Self::Repository => match &state.search_params.repo {
				Some(repo) => state
					.repositories
					.iter()
					.position(|y| y.id == *repo)
					.into_iter()
					.collect(),
				None => Vec::new(),
			},
			Self::PackageType => {
				let Some(ty) = state.search_params.inner.types.first() else {
					return Vec::new();
				};

				let Some(repo) = state.get_selected_repo_info() else {
					return Vec::new();
				};

				repo.metadata
					.package_types
					.iter()
					.position(|x| x == ty)
					.into_iter()
					.collect()
			}
			Self::Version => state
				.search_params
				.inner
				.minecraft_versions
				.iter()
				.filter_map(|x| state.versions.iter().position(|y| y == x))
				.collect(),
			Self::Loader => state
				.search_params
				.inner
				.loaders
				.iter()
				.filter_map(|x| state.loaders.iter().position(|y| y == x))
				.collect(),
			Self::Category => {
				let Some(repo) = state.get_selected_repo_info() else {
					return Vec::new();
				};

				state
					.search_params
					.inner
					.categories
					.iter()
					.filter_map(|x| repo.metadata.package_categories.iter().position(|y| y == x))
					.collect()
			}
		}
	}

	fn get_items(&self, state: &State) -> Vec<String> {
		match self {
			Self::Repository => state.repositories.iter().map(|x| x.id.clone()).collect(),
			Self::PackageType => {
				let Some(repo) = state.get_selected_repo_info() else {
					return Vec::new();
				};

				repo.metadata
					.package_types
					.iter()
					.map(|x| x.to_string())
					.collect()
			}
			Self::Version => state.versions.clone(),
			Self::Loader => state.loaders.iter().map(|x| x.to_string()).collect(),
			Self::Category => {
				let Some(repo) = state.get_selected_repo_info() else {
					return Vec::new();
				};

				repo.metadata
					.package_categories
					.iter()
					.map(|x| to_string_json(x))
					.collect()
			}
		}
	}

	fn title(&self) -> &'static str {
		match self {
			Self::Repository => "Select repository",
			Self::PackageType => "Select package type",
			Self::Version => "Select Minecraft versions",
			Self::Loader => "Select loaders",
			Self::Category => "Select categories",
		}
	}
}

/// Search parameters including repository
#[derive(Clone)]
struct SearchParams {
	inner: PackageSearchParameters,
	repo: Option<String>,
}

/// Info about a package
struct PackageInfo {
	meta: Arc<PackageMetadata>,
}

/// Gets a key
fn get_key() -> anyhow::Result<Option<KeyEvent>> {
	if !event::poll(Duration::from_millis(10)).context("Event poll failed")? {
		return Ok(None);
	}

	let event = event::read()
		.context("event read failed")?
		.as_key_press_event();
	let Some(event) = event else {
		return Ok(None);
	};

	Ok(Some(event))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewTab {
	Description,
	Versions,
	Gallery,
}

impl PreviewTab {
	fn title(&self) -> &'static str {
		match self {
			Self::Description => "Description [d]",
			Self::Versions => "Versions [v]",
			Self::Gallery => "Gallery [g]",
		}
	}
}

struct PackageInfoWidget<'a> {
	req: ArcPkgReq,
	info: Option<&'a PackageInfo>,
	state: &'a State<'a>,
	scroll_height: &'a mut u16,
}

impl<'a> Widget for PackageInfoWidget<'a> {
	fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
		let Some(info) = self.info else {
			Clear.render(area, buf);
			return;
		};

		let layout = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]);
		let layout = area.layout::<2>(&layout);
		let top_pane = layout[0];
		let bottom_pane = layout[1];

		// Top pane
		let block = Block::new().borders(Borders::BOTTOM);
		let inner_area = block.inner(top_pane);
		block.render(top_pane, buf);

		let layout = Layout::horizontal([Constraint::Length(6), Constraint::Fill(1)]);
		let layout = inner_area.layout::<2>(&layout);
		let icon_pane = layout[0];
		let details_pane = layout[1].inner(Margin::new(1, 1));

		// Icon
		if let Some(icon) = &info.meta.icon {
			if let Some(image) = self.state.image_cache.get_from_cache(icon) {
				let picker = Picker::from_query_stdio().unwrap_or(Picker::halfblocks());
				let image = (*image).clone();
				if let Ok(image) = picker.new_protocol(
					DynamicImage::ImageRgb8(image),
					icon_pane,
					Resize::Scale(None),
				) {
					let image = Image::new(&image);
					image.render(icon_pane, buf);
				}
			} else {
				self.state.request_image(icon);
			}
		}

		// Details
		let layout = Layout::horizontal(Constraint::from_fills([1, 1, 1]));
		let [title_pane, _, _] = details_pane.layout::<3>(&layout);

		// Title
		let title_name = if let Some(name) = &info.meta.name {
			name.as_str()
		} else if let Some(slug) = &info.meta.slug {
			slug.as_str()
		} else {
			&self.req.id
		};

		let title = if let Some(repo) = &self.req.repository {
			format!("{title_name} - {repo}")
		} else {
			title_name.to_string()
		};

		let title = Paragraph::new(title).style(Style::new().bold());
		title.render(title_pane, buf);

		// Subtitle
		let mut subtitle_area = details_pane;
		subtitle_area.y += 1;

		if let Some(short_description) = &info.meta.description {
			let short_description = Paragraph::new(short_description.as_str());
			short_description
				.style(Style::new().gray())
				.render(subtitle_area, buf);
		}

		// Bottom pane
		let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
		let layout = bottom_pane.layout::<2>(&layout);
		let tabs_pane = layout[0];
		let body_pane = layout[1].inner(Margin::new(1, 1));

		// Tabs
		let layout = Layout::horizontal(Constraint::from_fills([1, 1, 1]));
		let layout = tabs_pane.layout::<3>(&layout);

		for (i, id) in [
			PreviewTab::Description,
			PreviewTab::Versions,
			PreviewTab::Gallery,
		]
		.into_iter()
		.enumerate()
		{
			let mut tab = Paragraph::new(id.title()).alignment(HorizontalAlignment::Center);
			if self.state.preview_tab == id {
				tab = tab.style(Style::new().reversed());
			}
			tab.render(layout[i], buf);
		}

		// Body
		match self.state.preview_tab {
			PreviewTab::Description => {
				if let Some(body) = &info.meta.long_description {
					let text = tui_markdown::from_str(body);
					*self.scroll_height = text.lines.len() as u16;
					let markdown = Paragraph::new(text).scroll((self.state.preview_scroll, 0));
					markdown.render(body_pane, buf);
				}
			}
			PreviewTab::Versions => {
				Clear.render(body_pane, buf);
			}
			PreviewTab::Gallery => {
				Clear.render(body_pane, buf);
				if let Some(gallery) = &info.meta.gallery {
					*self.scroll_height = render_gallery(gallery, self.state, body_pane, buf);
				}
			}
		}
	}
}

/// Renders a package gallery, returning the scroll height
fn render_gallery(
	gallery: &[String],
	state: &State,
	area: Rect,
	buf: &mut ratatui::prelude::Buffer,
) -> u16 {
	const WIDTH: usize = 3;
	const HEIGHT: usize = 4;

	let picker = Picker::from_query_stdio().unwrap_or(Picker::halfblocks());

	let vertical_layout = Layout::vertical(Constraint::from_fills([1; HEIGHT]));
	for (row_i, row) in vertical_layout.split(area).into_iter().enumerate() {
		let horizontal_layout = Layout::horizontal(Constraint::from_fills([1; WIDTH]));
		let horizontal_layout = horizontal_layout.split(*row);

		let start = (row_i + state.preview_scroll as usize) * WIDTH;
		let end = start + WIDTH;

		let mut col = 0;
		for i in start..end {
			if i >= gallery.len() {
				continue;
			}

			let area = horizontal_layout[col];
			let url = &gallery[i];

			if let Some(image) = state.image_cache.get_from_cache(url) {
				let image = (*image).clone();
				if let Ok(image) =
					picker.new_protocol(DynamicImage::ImageRgb8(image), area, Resize::Scale(None))
				{
					let image = Image::new(&image);
					image.render(area, buf);
				}
			} else {
				state.request_image(url);
			}

			col += 1;
		}
	}

	// Account for trailing incomplete rows
	let scroll_height = if gallery.len() % WIDTH == 0 {
		gallery.len() / WIDTH
	} else {
		gallery.len() / WIDTH + 1
	};

	// Scroll height is equal to how much we need to go beyond the normally visible contents
	if scroll_height > HEIGHT {
		(scroll_height - HEIGHT) as u16
	} else {
		0
	}
}
