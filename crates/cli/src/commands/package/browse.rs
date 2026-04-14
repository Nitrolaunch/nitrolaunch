use std::time::Duration;

use anyhow::Context;
use crossterm::event::{self, KeyCode, KeyEvent};
use nitrolaunch::{
	config::Config,
	io::paths::Paths,
	pkg_crate::PackageSearchResults,
	plugin_crate::hook::hooks::{AddCustomPackageRepositories, AddCustomPackageRepositoriesResult},
	shared::{output::NoOp, pkg::PackageSearchParameters},
};
use ratatui::{
	layout::{Constraint, Layout, Margin, Rect},
	style::Style,
	widgets::{Block, Clear, List, ListState, Paragraph},
	DefaultTerminal, Frame,
};
use ratatui_textarea::TextArea;
use reqwest::Client;
use tokio::{
	sync::mpsc::{Receiver, Sender},
	task::JoinHandle,
};

use crate::commands::CmdData;

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
		// Check for results or state updates
		if let Ok(update) = state.worker_state_rx.try_recv() {
			state.worker_state = update;
			terminal.draw(|frame| render(frame, &mut state))?;
		}

		if let Ok(results) = state.results_rx.try_recv() {
			state.results = results;
			terminal.draw(|frame| render(frame, &mut state))?;
		}

		// Key checks. If there is no key event, no need to re-render
		let key = get_key()?;
		let Some(key) = key else {
			continue;
		};

		match state.focus {
			FocusState::None => match key.code {
				KeyCode::Char('q') => break,
				KeyCode::Char('s') | KeyCode::Char('/') => state.focus = FocusState::Search,
				KeyCode::Char('r') => state.focus = FocusState::Popup(Popup::Repository),
				KeyCode::Char('t') => state.focus = FocusState::Popup(Popup::PackageType),
				_ => {}
			},
			FocusState::Search => match key.code {
				KeyCode::Esc => {
					state.focus = FocusState::None;
				}
				KeyCode::Enter => {
					if let Some(line) = state.search.lines().first() {
						if line.is_empty() {
							state.search_params.inner.search = None;
						} else {
							state.search_params.inner.search = Some(line.clone());
						}
					}
					state.search();
				}
				_ => {
					state.search.input(key);
				}
			},
			FocusState::Popup(popup) => match key.code {
				KeyCode::Esc | KeyCode::Char('q') => state.focus = FocusState::None,
				_ => popup.input(&mut state, key),
			},
		}

		// Re-render
		terminal.draw(|frame| render(frame, &mut state))?;
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
	let status_layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]);
	let status_layout = status_pane.layout::<2>(&status_layout);
	let keybinds_pane = status_layout[0];
	let state_pane = status_layout[1];

	let keybinds = Paragraph::new("q to quit; s to search;");
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
	let preview_layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(3)]);
	let preview_layout = preview_pane.layout::<2>(&preview_layout);
	let list_pane = preview_layout[0];
	let preview_pane = preview_layout[1];

	// Package list
	state.package_list = state
		.package_list
		.clone()
		.items(state.results.results.clone());
	let block = Block::bordered().title("Packages");
	let inner = block.inner(list_pane);
	frame.render_widget(block, list_pane);
	frame.render_widget(&state.package_list, inner);

	// Preview pane
	let block = Block::bordered().title("Preview");
	frame.render_widget(block, preview_pane);

	// Filters
	let filter_layout = Layout::horizontal(Constraint::from_fills([1, 1, 1]));
	let filter_layout = filters_pane.layout::<3>(&filter_layout);
	let repo_pane = filter_layout[0];
	let type_pane = filter_layout[1];

	let repo = Paragraph::new(
		state
			.search_params
			.repo
			.clone()
			.unwrap_or("All Repos".into()),
	)
	.style(Style::new().bold());
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
	/// Search parameters
	search_params: SearchParams,
	/// Search bar
	search: TextArea<'a>,
	/// List of packages
	package_list: List<'a>,
	/// Available repositories
	repositories: Vec<AddCustomPackageRepositoriesResult>,
	/// Current focus state
	focus: FocusState,
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
		let handle = tokio::spawn(worker_thread(config, paths, state_tx, task_rx, results_tx));

		// Search bar
		let mut search = TextArea::new(Vec::new());
		search.set_style(Style::new().white());
		search.set_placeholder_text("Enter search...");
		let search_block = Block::bordered().title("Search");
		search.set_block(search_block);

		// Package list
		let package_list = List::default();

		Ok(Self {
			worker: handle,
			worker_state: WorkerState::Idle,
			worker_state_rx: state_rx,
			task_tx,
			results_rx,
			results: PackageSearchResults::default(),
			search_params: SearchParams {
				inner: PackageSearchParameters {
					count: 25,
					skip: 0,
					..Default::default()
				},
				repo: None,
			},
			search,
			package_list,
			repositories,
			focus: FocusState::None,
		})
	}

	/// Sends a request to search for packages given the current parameters
	fn search(&mut self) {
		let _ = self
			.task_tx
			.try_send(Task::FetchPackages(self.search_params.clone()));
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
		}
	}
}

/// Task that the worker thread can run
enum Task {
	FetchPackages(SearchParams),
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
			KeyCode::Up if pos > 0 => self.select(state, pos - 1),
			KeyCode::Down if pos < self.get_option_count(state) => self.select(state, pos + 1),
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
