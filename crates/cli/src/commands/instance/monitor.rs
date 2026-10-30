use std::{
	collections::HashMap,
	time::{Duration, Instant},
};

use anyhow::Context;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use nitrolaunch::{
	config::Config,
	instance::{
		operations::{LOCAL_INSTANCE_OUTPUT_UPDATE_RATE, REMOTE_INSTANCE_OUTPUT_UPDATE_RATE},
		tracking::{REMOTE_STATUS_UPDATE_INTERVAL, RunningInstanceRegistry},
	},
	io::paths::Paths,
	shared::{id::InstanceID, output::NoOp},
};
use ratatui::{
	DefaultTerminal, Frame,
	layout::{Constraint, HorizontalAlignment, Layout, Rect},
	style::{Color, Modifier, Style, Stylize},
	symbols,
	text::{Line, Span},
	widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Widget},
};
use ratatui_textarea::TextArea;
use tokio::{
	sync::mpsc::{Receiver, Sender},
	task::JoinHandle,
};

use crate::{commands::CmdData, prompt::get_key};

const SPINNER_INTERVAL: Duration = Duration::from_millis(300);
const SPINNER_FRAMES: [&str; 4] = ["⡈", "⠔", "⠢", "⢁"];

pub async fn run(mut data: CmdData<'_>) -> anyhow::Result<()> {
	data.ensure_config(false).await?;

	let instances: Vec<_> = data
		.config
		.get()
		.instances
		.keys()
		.cloned()
		.sorted()
		.collect();

	let state = State::new(data.config.take().unwrap(), data.paths.clone(), instances)?;

	ratatui::run(move |terminal| renderer(terminal, state)).context("Failed to run app")
}

/// State for the application
struct State<'a> {
	/// Handle for worker thread
	worker: JoinHandle<()>,
	/// Sends tasks to the worker thread
	output_tx: Sender<Task>,
	/// Receiver for instance output updates
	output_rx: Receiver<InstanceOutput>,
	/// Receiver for instance status updates
	status_rx: Receiver<Vec<InstanceID>>,
	/// Available instances
	instances: Vec<InstanceID>,
	/// Output for each instance
	output: HashMap<InstanceID, InstanceOutput>,
	/// The instances that are currently running
	running_instances: Vec<InstanceID>,
	/// Current focus state for the app
	focus: FocusState,
	/// Input field to send to instances
	input: TextArea<'a>,
	/// List of instances to display
	instance_list: List<'a>,
	/// State for the instance list widget
	instance_list_state: ListState,
	/// Last selected instance
	last_selected_instance: Option<InstanceID>,
	/// Scroll from the bottom of the console. 0 will auto-scroll.
	console_scroll: u64,
	/// Height of the console area for scrolling purposes
	console_scroll_height: u64,
	/// Horizontal scroll from the left of the console
	console_horizontal_scroll: u16,
	/// History of input commands. The last entry is the current input.
	input_history: Vec<String>,
	/// Index of the current input history entry
	input_history_index: usize,
	/// Current filter applied to messages
	filter: Filter,
	/// Current frame for running instance indicators
	spinner_stage: usize,
	/// Time when the running instance indicators last advanced
	last_spinner_update: Instant,
}

impl<'a> State<'a> {
	/// Initialize state with widgets
	fn new(config: Config, paths: Paths, instances: Vec<InstanceID>) -> anyhow::Result<Self> {
		// Setup worker
		let (output_tx, output_rx) = tokio::sync::mpsc::channel(4);
		let (focus_tx, focus_rx) = tokio::sync::mpsc::channel(4);
		let (status_tx, status_rx) = tokio::sync::mpsc::channel(4);
		let handle = tokio::spawn(worker_thread(config, paths, output_tx, focus_rx, status_tx));

		// Input bar
		let mut input = TextArea::new(Vec::new());
		input.set_style(Style::new().white());
		let input_block = rounded_block().title("Input");
		input.set_block(input_block);

		// Instance list
		let instance_list = List::default()
			.highlight_style(Style::new().green())
			.highlight_symbol(">");
		let mut instance_list_state = ListState::default();
		instance_list_state.select_first();

		let mut out = Self {
			worker: handle,
			status_rx,
			output_tx: focus_tx,
			output_rx,
			instances,
			output: HashMap::new(),
			running_instances: Vec::new(),
			focus: FocusState::List,
			input,
			instance_list,
			instance_list_state,
			last_selected_instance: None,
			console_scroll: 0,
			console_scroll_height: 0,
			console_horizontal_scroll: 0,
			input_history: vec![String::new()],
			input_history_index: 0,
			filter: Filter::All,
			spinner_stage: 0,
			last_spinner_update: Instant::now(),
		};

		out.select_instance();

		Ok(out)
	}

	fn focus_out(&mut self) {
		match self.focus {
			FocusState::Input => self.focus = FocusState::Console,
			FocusState::Console => self.focus = FocusState::List,
			_ => {}
		}
	}

	fn get_selected_instance(&self) -> Option<InstanceID> {
		self.instance_list_state
			.selected()
			.and_then(|i| self.instances.get(i))
			.cloned()
	}

	fn send_input(&mut self) {
		let text = self.input.lines().join("\n");
		let _ = self.output_tx.try_send(Task::SendInput {
			instance_id: self.get_selected_instance().unwrap_or_default(),
			input: format!("{text}\n"),
		});
		*self.input_history.last_mut().unwrap() = text;
		self.input_history.push(String::new());
		self.input.clear();
		self.input_history_index = self.input_history.len() - 1;
	}

	fn set_input(&mut self, text: String) {
		self.input.clear();
		for c in text.chars() {
			self.input
				.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
		}
	}

	/// Selects the currently highlighted instance in the list and sets it as the focus
	fn select_instance(&mut self) {
		let Some(instance_id) = self.get_selected_instance() else {
			return;
		};

		if Some(instance_id.clone()) == self.last_selected_instance {
			return;
		}
		self.last_selected_instance = Some(instance_id.clone());
		self.console_scroll = 0;

		let _ = self.output_tx.try_send(Task::Focus(Focus {
			instance_id: instance_id.clone(),
			after: self
				.output
				.get(&instance_id)
				.map(|x| x.output.len() as u64)
				.unwrap_or_default(),
		}));
	}

	fn set_filter(&mut self, filter: Filter) {
		self.filter = filter;
		self.console_scroll = 0;
	}
}

impl<'a> Drop for State<'a> {
	fn drop(&mut self) {
		self.worker.abort();
	}
}

/// Main event loop
fn renderer(terminal: &mut DefaultTerminal, mut state: State<'_>) -> anyhow::Result<()> {
	// Initial draw
	terminal.draw(|frame| render(frame, &mut state))?;

	loop {
		let mut should_render = false;
		if state.last_spinner_update.elapsed() >= SPINNER_INTERVAL {
			state.spinner_stage = (state.spinner_stage + 1) % SPINNER_FRAMES.len();
			state.last_spinner_update = Instant::now();
			should_render = true;
		}

		// Check for results or state updates
		if let Ok(output) = state.output_rx.try_recv() {
			state
				.output
				.entry(output.instance_id.clone())
				.or_insert_with(|| InstanceOutput {
					instance_id: output.instance_id.clone(),
					output: String::new(),
				})
				.output
				.push_str(&output.output);
			should_render = true;
		}

		if let Ok(running_instances) = state.status_rx.try_recv() {
			state.running_instances = running_instances;
			should_render = true;
		}

		// Key checks. If there is no key event, no need to re-render
		let key = get_key()?;
		if let Some(key) = key {
			should_render = true;

			match state.focus {
				_ if key.code == KeyCode::Esc => state.focus_out(),
				_ if key.code == KeyCode::Char('q') && state.focus != FocusState::Input => break,
				FocusState::List => match key.code {
					KeyCode::Up | KeyCode::Char('k') => {
						state.instance_list_state.select_previous();
						state.select_instance();
					}
					KeyCode::Down | KeyCode::Char('j') => {
						state.instance_list_state.select_next();
						state.select_instance();
					}
					KeyCode::Enter | KeyCode::Tab => {
						state.focus = FocusState::Console;
					}
					_ => {}
				},
				FocusState::Console => match key.code {
					KeyCode::Tab => state.focus = FocusState::List,
					KeyCode::Enter => state.focus = FocusState::Input,
					KeyCode::Up | KeyCode::Char('k') => {
						if state.console_scroll < state.console_scroll_height {
							state.console_scroll += 1;
						}
					}
					KeyCode::Down | KeyCode::Char('j') => {
						if state.console_scroll > 0 {
							state.console_scroll -= 1;
						}
					}
					KeyCode::Left | KeyCode::Char('h') => {
						if state.console_horizontal_scroll > 0 {
							state.console_horizontal_scroll -= 1;
						}
					}
					KeyCode::Right | KeyCode::Char('l') => {
						state.console_horizontal_scroll += 1;
					}
					KeyCode::Char(' ') => {
						state.console_scroll = 0;
						state.console_horizontal_scroll = 0;
					}
					KeyCode::Char('a') => state.set_filter(Filter::All),
					KeyCode::Char('i') => state.set_filter(Filter::Info),
					KeyCode::Char('w') => state.set_filter(Filter::Warn),
					KeyCode::Char('e') => state.set_filter(Filter::Error),
					KeyCode::Char('c') => state.set_filter(Filter::Chat),
					_ => {}
				},
				FocusState::Input => match key.code {
					KeyCode::Enter => {
						let line = state.input.lines().first().unwrap();
						if !line.is_empty() {
							state.send_input();
						}
					}
					KeyCode::Up => {
						if state.input_history_index > 0 {
							state.input_history_index -= 1;
							state.set_input(state.input_history[state.input_history_index].clone());
						}
					}
					KeyCode::Down => {
						if state.input_history_index < state.input_history.len() - 1 {
							state.input_history_index += 1;
							state.set_input(state.input_history[state.input_history_index].clone());
						}
					}
					_ => {
						state.input.input(key);
						if let Some(last) = state.input_history.last_mut() {
							*last = state.input.lines().join("\n");
						}
					}
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
fn render<'a>(frame: &mut Frame, state: &mut State<'a>) {
	let layout = Layout::vertical([
		Constraint::Length(1),
		Constraint::Fill(1),
		Constraint::Length(1),
	]);
	let layout = frame.area().layout::<3>(&layout);
	let header_pane = layout[0];
	let main_pane = layout[1];
	let status_pane = layout[2];

	// Draw panes

	// Status bar

	let keybinds_text = match state.focus {
		FocusState::List => "q to quit; k/j for up/down; tab/enter to focus console",
		FocusState::Console => {
			"esc to return to list; h/k/j/l for scroll; enter to focus input; space to jump to bottom; a/i/w/e/c for filter"
		}
		FocusState::Input => "esc to exit search; enter to submit; up/down to navigate history",
	};

	let keybinds = Paragraph::new(keybinds_text);
	frame.render_widget(keybinds, status_pane);

	// Main layout
	let main_layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(4)]);
	let main_layout = main_pane.layout::<2>(&main_layout);
	let list_pane = main_layout[0];
	let console_pane = main_layout[1];

	// Instance list
	let instance_items = state.instances.iter().map(|x| {
		let is_running = state.running_instances.contains(x);
		let (indicator, indicator_style) = if is_running {
			(
				format!("{} ", SPINNER_FRAMES[state.spinner_stage]),
				Style::new().yellow(),
			)
		} else {
			("○ ".to_owned(), Style::new().dark_gray())
		};
		ListItem::new(Line::from(vec![
			Span::styled(indicator, indicator_style),
			Span::raw(x.to_string()),
		]))
	});
	state.instance_list = state.instance_list.clone().items(instance_items);
	let mut block = rounded_block().title("Instances");
	if state.focus == FocusState::List {
		block = block.border_style(Style::new().green());
	}
	let inner = block.inner(list_pane);
	frame.render_widget(block, list_pane);
	frame.render_stateful_widget(&state.instance_list, inner, &mut state.instance_list_state);

	// Console pane
	let mut block = rounded_block().title("Console");
	if state.focus == FocusState::Console {
		block = block.border_style(Style::new().green());
	}
	let inner_area = block.inner(console_pane);
	frame.render_widget(block, console_pane);
	if let Some(instance_id) = state.get_selected_instance() {
		let mut scroll_height = 0;
		let widget = ConsoleWidget {
			instance_id,
			state,
			scroll_height: &mut scroll_height,
		};

		frame.render_widget(widget, inner_area);
		state.console_scroll_height = scroll_height;
	} else {
		frame.render_widget(Clear, inner_area);
	};

	// Header pane
	let layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]);
	let layout = header_pane.layout::<2>(&layout);
	let left_header_pane = layout[0];
	let filter_pane = layout[1];
	let text = Line::from("Nitrolaunch");
	frame.render_widget(text, left_header_pane);

	let filters = [
		Filter::All,
		Filter::Info,
		Filter::Warn,
		Filter::Error,
		Filter::Chat,
	];
	let filters = filters.iter().map(|x| {
		if state.filter == *x {
			x.to_span().underlined()
		} else {
			x.to_span()
		}
	});
	let text = Line::from(filters.collect::<Vec<_>>()).alignment(HorizontalAlignment::Right);
	frame.render_widget(text, filter_pane);
}

/// Focus state for the TUI
#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusState {
	List,
	Console,
	Input,
}

struct ConsoleWidget<'a, 'b, 'c> {
	instance_id: InstanceID,
	state: &'a mut State<'b>,
	scroll_height: &'c mut u64,
}

impl<'a, 'b, 'c> Widget for ConsoleWidget<'a, 'b, 'c> {
	fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
		let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]);
		let layout = area.layout::<2>(&layout);
		let output_pane = layout[0];
		let input_pane = layout[1];

		// Output pane
		let block = Block::new().borders(Borders::BOTTOM);
		let inner_area = block.inner(output_pane);
		block.render(output_pane, buf);

		// Output
		let default = InstanceOutput {
			instance_id: self.instance_id.clone(),
			output: "Loading...".into(),
		};
		let output_text = self.state.output.get(&self.instance_id).unwrap_or(&default);
		let visible_lines = inner_area.height as u64;

		// We do our own scrolling
		let mut scroll_height = 0;
		let mut lines = Vec::new();
		for line in output_text.output.lines().rev() {
			let matches = self.state.filter.check_line(line);
			if !matches {
				continue;
			}
			scroll_height += 1;
			if scroll_height >= self.state.console_scroll
				&& scroll_height < self.state.console_scroll + visible_lines
			{
				let line = format_instance_stdout_line_ratatui(line);
				lines.insert(0, line);
			}
		}
		let scroll_height = scroll_height.saturating_sub(visible_lines);
		*self.scroll_height = scroll_height;

		let output = Paragraph::new(lines).scroll((0, self.state.console_horizontal_scroll));
		output.render(inner_area, buf);

		// Input
		let input_block = self.state.input.block().unwrap();
		let input_block_style = if self.state.focus == FocusState::Input {
			Style::new().green()
		} else {
			Style::new().gray()
		};
		let input_block = input_block.clone().border_style(input_block_style);
		self.state.input.set_block(input_block);
		self.state.input.render(input_pane, buf);
	}
}

/// Formats an output line as ratatui text instead of terminal escape sequences
fn format_instance_stdout_line_ratatui(line: &str) -> Line<'static> {
	if line.starts_with('\u{001b}') {
		return Line::raw(line.to_owned());
	}

	let mut spans = Vec::new();
	let (timestamp, remainder) = if line.starts_with('[') {
		line.find(']')
			.map(|end| (&line[..=end], &line[end + 1..]))
			.unwrap_or(("", line))
	} else {
		("", line)
	};

	if !timestamp.is_empty() {
		spans.push(Span::styled(timestamp.to_owned(), Style::new().dark_gray()));
	}

	let levels = [
		("/INFO", Style::new().dark_gray()),
		("/WARN", Style::new().yellow()),
		("/ERROR", Style::new().red()),
	];
	let Some((level_start, (level, level_style))) = levels
		.iter()
		.filter_map(|level| remainder.find(level.0).map(|start| (start, level)))
		.min_by_key(|(start, _)| *start)
	else {
		spans.extend(format_section_codes(remainder));
		return Line::from(spans);
	};

	spans.push(Span::raw(remainder[..level_start].to_owned()));
	spans.push(Span::styled((*level).to_owned(), *level_style));
	spans.extend(format_section_codes(
		&remainder[level_start + level.len()..],
	));
	Line::from(spans)
}

/// Converts Minecraft section codes in a message into ratatui spans.
fn format_section_codes(text: &str) -> Vec<Span<'static>> {
	let mut spans = Vec::new();
	let mut style = Style::default();
	let mut content = String::new();

	let push_content = |spans: &mut Vec<Span<'static>>, content: &mut String, style: Style| {
		if !content.is_empty() {
			spans.push(Span::styled(std::mem::take(content), style));
		}
	};

	let mut chars = text.chars();
	while let Some(character) = chars.next() {
		if character != '§' {
			content.push(character);
			continue;
		}

		let Some(code) = chars.next() else {
			content.push(character);
			break;
		};

		let Some(new_style) = section_style(code, style) else {
			content.push(character);
			content.push(code);
			continue;
		};

		push_content(&mut spans, &mut content, style);
		style = new_style;
	}

	push_content(&mut spans, &mut content, style);
	spans
}

fn section_style(code: char, current: Style) -> Option<Style> {
	let color = match code.to_ascii_lowercase() {
		'0' => Some(Color::Black),
		'1' => Some(Color::Rgb(0, 0, 170)),
		'2' => Some(Color::Rgb(0, 170, 0)),
		'3' => Some(Color::Rgb(0, 170, 170)),
		'4' => Some(Color::Rgb(170, 0, 0)),
		'5' => Some(Color::Rgb(170, 0, 170)),
		'6' => Some(Color::Yellow),
		'7' => Some(Color::Gray),
		'8' => Some(Color::DarkGray),
		'9' => Some(Color::Blue),
		'a' => Some(Color::Green),
		'b' => Some(Color::Cyan),
		'c' => Some(Color::Red),
		'd' => Some(Color::Magenta),
		'e' => Some(Color::Yellow),
		'f' => Some(Color::White),
		_ => None,
	};

	if let Some(color) = color {
		return Some(Style::default().fg(color));
	}

	match code.to_ascii_lowercase() {
		'k' => Some(current.add_modifier(Modifier::RAPID_BLINK)),
		'l' => Some(current.add_modifier(Modifier::BOLD)),
		'm' => Some(current.add_modifier(Modifier::CROSSED_OUT)),
		'n' => Some(current.add_modifier(Modifier::UNDERLINED)),
		'o' => Some(current.add_modifier(Modifier::ITALIC)),
		'r' => Some(Style::default()),
		_ => None,
	}
}

/// Loop for tokio task that does tasks like fetching output, sending input, and checking status
async fn worker_thread(
	config: Config,
	paths: Paths,
	output_tx: Sender<InstanceOutput>,
	mut task_rx: Receiver<Task>,
	status_tx: Sender<Vec<InstanceID>>,
) {
	let mut focus = None;
	let mut output_interval = tokio::time::interval(LOCAL_INSTANCE_OUTPUT_UPDATE_RATE);
	let mut local_status_interval = tokio::time::interval(Duration::from_millis(150));
	let mut remote_status_interval = tokio::time::interval(REMOTE_STATUS_UPDATE_INTERVAL);
	let mut registry = RunningInstanceRegistry::open(&paths).unwrap();

	loop {
		tokio::select! {
			Some(task) = task_rx.recv() => {
				match task {
					Task::Focus(new_focus) => {
						if let Some(instance) = config.instances.get(&new_focus.instance_id) {
							if instance.config().is_remote {
								output_interval = tokio::time::interval(REMOTE_INSTANCE_OUTPUT_UPDATE_RATE);
							} else {
								output_interval = tokio::time::interval(LOCAL_INSTANCE_OUTPUT_UPDATE_RATE);
							}
						}
						focus = Some(new_focus);
					}
					Task::SendInput { instance_id, input } => {
						if let Some(instance) = config.instances.get(&instance_id) {
							let _ = instance.send_input(&input, &paths, &config.plugins, &mut NoOp).await;
						}
					}
				}
			}
			_ = output_interval.tick() => {
				let Some(focus) = &mut focus else {
					continue;
				};
				let Some(instance) = config.instances.get(&focus.instance_id) else {
					continue;
				};
				if let Ok(output) = instance.get_output(focus.after, &paths, &config.plugins, &mut NoOp).await {
					focus.after += output.len() as u64;
					let _ = output_tx.try_send(InstanceOutput {
						instance_id: focus.instance_id.clone(),
						output,
					});
				}
			}
			_ = local_status_interval.tick() => {
				let _ = registry.reread();
				let _ = registry.update_from_plugins(false, &config.plugins, &paths, &mut NoOp).await;
				let _ = status_tx.try_send(registry.iter_entries().map(|x| x.instance_id.clone().into()).collect());
				let _ = registry.write();
			}
			_ = remote_status_interval.tick() => {
				let _ = registry.reread();
				let _ = registry.update_from_plugins(true, &config.plugins, &paths, &mut NoOp).await;
				let _ = status_tx.try_send(registry.iter_entries().map(|x| x.instance_id.clone().into()).collect());
				let _ = registry.write();
			}
		}
	}
}

/// Task for the worker to work on
enum Task {
	Focus(Focus),
	SendInput {
		instance_id: InstanceID,
		input: String,
	},
}

struct Focus {
	instance_id: InstanceID,
	after: u64,
}

/// Update for output received from an instance
struct InstanceOutput {
	instance_id: InstanceID,
	output: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
	All,
	Info,
	Warn,
	Error,
	Chat,
}

impl Filter {
	fn style(&self) -> Style {
		match self {
			Filter::All => Style::new().white(),
			Filter::Info => Style::new().gray(),
			Filter::Warn => Style::new().yellow(),
			Filter::Error => Style::new().red(),
			Filter::Chat => Style::new().blue(),
		}
	}

	fn to_span(&self) -> Span<'static> {
		let text = match self {
			Filter::All => "all ",
			Filter::Info => "info ",
			Filter::Warn => "warn ",
			Filter::Error => "error ",
			Filter::Chat => "chat ",
		};
		Span::raw(text).style(self.style())
	}

	fn check_line(&self, line: &str) -> bool {
		match self {
			Filter::All => true,
			Filter::Info => line.contains("INFO"),
			Filter::Warn => line.contains("WARN"),
			Filter::Error => line.contains("ERR"),
			Filter::Chat => line.contains("CHAT"),
		}
	}
}

fn rounded_block<'a>() -> Block<'a> {
	Block::bordered().border_set(symbols::border::ROUNDED)
}
