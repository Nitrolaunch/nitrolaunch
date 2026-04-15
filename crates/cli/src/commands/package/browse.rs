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
	io::paths::Paths,
	pkg_crate::{metadata::PackageMetadata, PackageSearchResults, PkgRequest, PkgRequestSource},
	plugin_crate::hook::hooks::{AddCustomPackageRepositories, AddCustomPackageRepositoriesResult},
	shared::{
		output::NoOp,
		pkg::{ArcPkgReq, PackageSearchParameters},
	},
};
use ratatui::{
	layout::{Constraint, Layout, Margin, Rect},
	style::Style,
	widgets::{Block, Borders, Clear, List, ListState, Paragraph, StatefulWidget, Widget},
	DefaultTerminal, Frame,
};
use ratatui_image::{picker::Picker, StatefulImage};
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

	ratatui::run(move |terminal| {
		renderer(
			terminal,
			data.config.take().unwrap(),
			data.paths.clone(),
			repos,
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
) -> anyhow::Result<()> {
	let mut state = State::new(config, paths, available_repos)?;

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
				_ if key.code == KeyCode::Esc => state.focus = FocusState::None,
				_ if key.code == KeyCode::Char('q') && state.focus != FocusState::Search => break,
				FocusState::None => match key.code {
					KeyCode::Char('s') | KeyCode::Char('/') => state.focus = FocusState::Search,
					KeyCode::Char('r') => state.focus = FocusState::Popup(Popup::Repository),
					KeyCode::Char('t') => state.focus = FocusState::Popup(Popup::PackageType),
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
						state.focus = FocusState::None;
					}
					_ => {
						state.search.input(key);
					}
				},
				FocusState::Popup(popup) => popup.input(&mut state, key),
				FocusState::Preview => match key.code {
					KeyCode::Char('p') | KeyCode::Tab => state.focus = FocusState::None,
					KeyCode::Up | KeyCode::Char('k') if state.preview_scroll > 0 => {
						state.preview_scroll -= 1
					}
					KeyCode::Down | KeyCode::Char('j') => state.preview_scroll += 1,
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
		let widget = PackageInfoWidget {
			req,
			info: state.package_info.as_ref(),
			state: state,
		};

		frame.render_widget(widget, inner_area);
	} else {
		frame.render_widget(Clear, inner_area);
	};

	// Filters
	let filter_layout = Layout::horizontal(Constraint::from_fills([1, 1, 1]));
	let filter_layout = filters_pane.layout::<3>(&filter_layout);
	let repo_pane = filter_layout[0];
	let type_pane = filter_layout[1];

	let repo = format!("Repository: {}", state
			.search_params
			.repo
			.as_deref()
			.unwrap_or("All"));
	let repo = Paragraph::new(repo).style(Style::new().bold());
	frame.render_widget(repo, repo_pane);

	let ty = Paragraph::new(
		state
			.search_params
			.inner
			.types
			.first()
			.map(|x| x.to_string())
			.unwrap_or("Any Type".into()),
	)
	.style(Style::new().bold());
	frame.render_widget(ty, type_pane);

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
	/// List of packages
	package_list: List<'a>,
	/// List state for package list
	package_list_state: ListState,
	/// Last selected package
	last_selected_package: Option<ArcPkgReq>,
	/// Available repositories
	repositories: Vec<AddCustomPackageRepositoriesResult>,
	/// Current scroll of preview pane body
	preview_scroll: u16,
}

impl<'a> State<'a> {
	/// Initialize state with widgets and worker thread
	fn new(
		config: Config,
		paths: Paths,
		repositories: Vec<AddCustomPackageRepositoriesResult>,
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
			search,
			package_list,
			package_list_state,
			last_selected_package: None,
			repositories,
			focus: FocusState::None,
			preview_scroll: 0,
		})
	}

	/// Gets the currently selected package
	fn get_selected_package(&self) -> Option<ArcPkgReq> {
		let pos = self.package_list_state.selected()?;
		let pkg = self.results.results.get(pos)?;

		Some(PkgRequest::parse(pkg, PkgRequestSource::UserRequire).arc())
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
}

impl Popup {
	fn render(&self, state: &mut State, frame: &mut Frame, area: Rect) {
		let layout = Layout::horizontal(Constraint::from_fills([1, 1, 1]));
		let layout = area.layout::<3>(&layout);

		let block = Block::bordered()
			.border_style(Style::new().green())
			.title(self.title());
		let list = List::default()
			.block(block)
			.items(self.get_items(state))
			.highlight_symbol(">")
			.highlight_style(Style::new().green());

		let area = match self {
			Self::Repository => layout[0],
			Self::PackageType => layout[1],
		};

		let mut list_state = ListState::default();
		list_state.select(self.get_select_position(state));

		frame.render_widget(Clear, area);
		frame.render_stateful_widget(list, area, &mut list_state);
	}

	fn input(&self, state: &mut State, input: KeyEvent) {
		let Some(pos) = self.get_select_position(state) else {
			return;
		};

		match input.code {
			KeyCode::Up | KeyCode::Char('k') if pos > 0 => self.select(state, pos - 1),
			KeyCode::Down | KeyCode::Char('j') if pos < self.get_option_count(state) => {
				self.select(state, pos + 1)
			}
			KeyCode::Enter => match self {
				Self::Repository | Self::PackageType => {
					self.select(state, pos);
					state.focus = FocusState::None;
					state.search();
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
				let Some(repo) = state
					.repositories
					.iter()
					.find(|x| Some(&x.id) == state.search_params.repo.as_ref())
				else {
					return;
				};
				let Some(ty) = repo.metadata.package_types.get(pos) else {
					return;
				};

				state.search_params.inner.types = vec![ty.clone()];
			}
		}
	}

	fn get_select_position(&self, state: &State) -> Option<usize> {
		match self {
			Self::Repository => match &state.search_params.repo {
				Some(repo) => state.repositories.iter().position(|y| y.id == *repo),
				None => Some(0),
			},
			Self::PackageType => {
				let Some(ty) = state.search_params.inner.types.first() else {
					return Some(0);
				};

				let Some(repo) = state
					.repositories
					.iter()
					.find(|x| Some(&x.id) == state.search_params.repo.as_ref())
				else {
					return None;
				};

				repo.metadata.package_types.iter().position(|x| x == ty)
			}
		}
	}

	fn get_option_count(&self, state: &State) -> usize {
		match self {
			Self::Repository => state.repositories.len(),
			Self::PackageType => {
				let Some(repo) = state
					.repositories
					.iter()
					.find(|x| Some(&x.id) == state.search_params.repo.as_ref())
				else {
					return 0;
				};

				repo.metadata.package_types.len()
			}
		}
	}

	fn get_items(&self, state: &State) -> Vec<String> {
		match self {
			Self::Repository => state.repositories.iter().map(|x| x.id.clone()).collect(),
			Self::PackageType => {
				let Some(repo) = state
					.repositories
					.iter()
					.find(|x| Some(&x.id) == state.search_params.repo.as_ref())
				else {
					return Vec::new();
				};

				repo.metadata
					.package_types
					.iter()
					.map(|x| x.to_string())
					.collect()
			}
		}
	}

	fn title(&self) -> &'static str {
		match self {
			Self::Repository => "Select repository",
			Self::PackageType => "Select package type",
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

struct PackageInfoWidget<'a> {
	req: ArcPkgReq,
	info: Option<&'a PackageInfo>,
	state: &'a State<'a>,
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
				let mut image = picker.new_resize_protocol(DynamicImage::ImageRgb8(image));
				let widget = StatefulImage::new();
				widget.render(icon_pane, buf, &mut image);
			} else {
				self.state.request_image(icon);
			}
		}

		// Details
		let layout = Layout::horizontal(Constraint::from_fills([1, 1, 1]));
		let layout = details_pane.layout::<3>(&layout);
		let title_pane = layout[0];

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
			short_description.style(Style::new().gray()).render(subtitle_area, buf);
		}

		// Body pane
		if let Some(body) = &info.meta.long_description {
			let markdown =
				Paragraph::new(tui_markdown::from_str(body)).scroll((self.state.preview_scroll, 0));
			markdown.render(bottom_pane.inner(Margin::new(1, 1)), buf);
		}
	}
}
