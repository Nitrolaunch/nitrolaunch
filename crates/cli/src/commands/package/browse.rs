use std::time::Duration;

use anyhow::Context;
use crossterm::event::{self, KeyCode, KeyEvent};
use nitrolaunch::{
	config::Config,
	io::paths::Paths,
	pkg_crate::PackageSearchResults,
	shared::{output::NoOp, pkg::PackageSearchParameters},
};
use ratatui::{
	layout::{Constraint, Layout},
	style::Style,
	widgets::{Block, List, Paragraph},
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

	ratatui::run(move |terminal| {
		renderer(terminal, data.config.take().unwrap(), data.paths.clone())
	})
	.context("Failed to run app")
}

/// Main event loop
fn renderer(terminal: &mut DefaultTerminal, config: Config, paths: Paths) -> anyhow::Result<()> {
	let mut state = State::new(config, paths);

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

		match key.code {
			KeyCode::Char('q') => break,
			KeyCode::Esc => state.search_focused = false,
			KeyCode::Char('/') => state.search_focused = !state.search_focused,
			KeyCode::Char('s') if !state.search_focused => state.search_focused = true,
			_ if state.search_focused => {
				// Search confirmation
				if key.code == KeyCode::Enter {
					if let Some(line) = state.search.lines().first() {
						if line.is_empty() {
							state.search_params.inner.search = None;
						} else {
							state.search_params.inner.search = Some(line.clone());
						}
					}
					state.search();
				} else {
					state.search.input(key);
				}
			}
			_ => {}
		}

		// Re-render
		terminal.draw(|frame| render(frame, &mut state))?;
	}
	Ok(())
}

/// Main render
fn render(frame: &mut Frame, state: &mut State) {
	let layout = Layout::vertical([
		Constraint::Fill(1),
		Constraint::Length(3),
		Constraint::Length(1),
	]);
	let layout = frame.area().layout::<3>(&layout);
	let preview_pane = layout[0];
	let search_pane = layout[1];
	let status_pane = layout[2];

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
	let search_block_style = if state.search_focused {
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
	/// Whether the search bar is focused
	search_focused: bool,
	/// List of packages
	package_list: List<'a>,
}

impl<'a> State<'a> {
	/// Initialize state with widgets and worker thread
	fn new(config: Config, paths: Paths) -> Self {
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

		Self {
			worker: handle,
			worker_state: WorkerState::Idle,
			worker_state_rx: state_rx,
			task_tx,
			results_rx,
			results: PackageSearchResults::default(),
			search_params: SearchParams {
				inner: PackageSearchParameters {
					count: 15,
					skip: 0,
					..Default::default()
				},
				repo: None,
			},
			search,
			search_focused: true,
			package_list,
		}
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
