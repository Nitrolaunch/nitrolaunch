use std::{
	collections::VecDeque,
	sync::{Arc, atomic::AtomicU64},
	time::Duration,
};

use dashmap::DashMap;
use nitro_net::download::Client;
use nitro_shared::output::{Message, MessageContents, MessageLevel, NitroOutput};
use nitrolaunch::io::{logging::Logger, paths::Paths};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};

/// NitroOutput implementation to output to the remote client. Can be many of these.
pub struct RemoteOutput {
	job: Option<u64>,
	jobs: Arc<DashMap<u64, Job>>,
	job_counter: Arc<AtomicU64>,
	tx: broadcast::Sender<InputEvent>,
	rx: broadcast::Receiver<InputEvent>,
	logger_tx: mpsc::Sender<Message>,
}

impl RemoteOutput {
	/// Creates a new RemoteOutput.
	/// Also starts up a logger task.
	pub fn new(paths: &Paths) -> Self {
		let (tx, rx) = tokio::sync::broadcast::channel(15);
		let (logger_tx, mut logger_rx) = tokio::sync::mpsc::channel::<Message>(150);

		if let Ok(mut logger) = Logger::new(paths, "remote") {
			tokio::spawn(async move {
				loop {
					let Some(message) = logger_rx.recv().await else {
						break;
					};

					let _ = logger.log_message(message.contents, message.level);
				}
			});
		} else {
			eprintln!("Failed to create logger");
		}

		Self {
			job: None,
			jobs: Arc::new(DashMap::new()),
			job_counter: Arc::new(AtomicU64::new(0)),
			logger_tx,
			tx,
			rx,
		}
	}

	pub fn set_job(&mut self, job_id: u64) {
		self.job = Some(job_id);
		self.jobs.entry(job_id).or_insert_with(|| Job {
			id: job_id,
			events: VecDeque::new(),
			is_finished: false,
		});
	}

	pub fn new_job(&mut self) {
		let job_id = self
			.job_counter
			.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		self.set_job(job_id);
	}

	pub fn job_id(&self) -> Option<u64> {
		self.job
	}

	pub fn get_job(&self, job_id: u64) -> Option<Job> {
		self.jobs.get(&job_id).map(|job| job.clone())
	}

	pub fn finish(&self) {
		if let Some(job_id) = self.job {
			self.finish_job(job_id);
		}
	}

	pub fn finish_job(&self, job_id: u64) {
		if let Some(mut job) = self.jobs.get_mut(&job_id) {
			job.is_finished = true;
		}
	}

	pub fn log_message(&self, message: Message) {
		let _ = self.logger_tx.try_send(message);
	}

	pub fn log(&self, contents: MessageContents) {
		let message = Message {
			contents,
			level: MessageLevel::Important,
		};
		self.log_message(message);
	}

	fn send_event(&self, event: OutputEvent) {
		if let Some(job_id) = self.job {
			self.jobs
				.entry(job_id)
				.or_insert_with(|| Job {
					id: job_id,
					events: VecDeque::new(),
					is_finished: false,
				})
				.events
				.push_back(event);
		}
	}
}

impl Clone for RemoteOutput {
	fn clone(&self) -> Self {
		Self {
			job: self.job,
			jobs: self.jobs.clone(),
			job_counter: self.job_counter.clone(),
			tx: self.tx.clone(),
			rx: self.rx.resubscribe(),
			logger_tx: self.logger_tx.clone(),
		}
	}
}

impl Drop for RemoteOutput {
	fn drop(&mut self) {
		self.finish();
	}
}

#[async_trait::async_trait]
impl NitroOutput for RemoteOutput {
	fn display_text(&mut self, text: String, level: nitro_shared::output::MessageLevel) {
		self.display_message(Message {
			contents: text.into(),
			level,
		});
	}

	fn display_message(&mut self, message: Message) {
		self.send_event(OutputEvent::Message(message.clone()));
		self.log_message(message.clone());
		println!("{}", message.contents.default_format());
	}

	fn start_process(&mut self) {
		self.send_event(OutputEvent::StartProcess);
	}

	fn end_process(&mut self) {
		self.send_event(OutputEvent::EndProcess);
	}

	fn start_section(&mut self) {
		self.send_event(OutputEvent::StartSection);
	}

	fn end_section(&mut self) {
		self.send_event(OutputEvent::EndSection);
	}

	async fn prompt_yes_no(
		&mut self,
		default: bool,
		message: MessageContents,
	) -> anyhow::Result<bool> {
		self.send_event(OutputEvent::PromptYesNo(message));
		match self.rx.recv().await {
			Ok(InputEvent::YesNo(value)) => Ok(value),
			_ => Ok(default),
		}
	}
}

#[derive(Clone)]
pub struct Job {
	pub id: u64,
	pub events: VecDeque<OutputEvent>,
	pub is_finished: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum OutputEvent {
	Message(Message),
	StartProcess,
	EndProcess,
	StartSection,
	EndSection,
	PromptYesNo(MessageContents),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum InputEvent {
	YesNo(bool),
}

/// Used on the client to listen for output events from the server
pub struct RemoteOutputListener {
	job: u64,
	remote_settings: crate::client::RemoteSettings,
	client: Client,
	current_event_index: usize,
}

impl RemoteOutputListener {
	pub fn new(job: u64, remote_settings: crate::client::RemoteSettings, client: Client) -> Self {
		Self {
			job,
			remote_settings,
			client,
			current_event_index: 0,
		}
	}

	/// Polls the remote server for new output events for this job. Returns the events and whether the job is finished.
	pub async fn poll(&mut self) -> anyhow::Result<(Vec<OutputEvent>, bool)> {
		let response =
			crate::client::get_job(&self.remote_settings, &self.client, self.job).await?;
		let Some(response) = response else {
			return Ok((Vec::new(), true));
		};

		let events = response.events;
		if self.current_event_index >= events.len() {
			Ok((Vec::new(), response.is_finished))
		} else {
			let new_events = events[self.current_event_index..].to_vec();
			self.current_event_index = events.len();
			Ok((new_events, response.is_finished))
		}
	}

	/// Listens for output events from the remote server and applies them to the given NitroOutput. This will block until the job is complete.
	pub async fn listen(&mut self, o: &mut impl NitroOutput) {
		loop {
			let result = self.poll().await;
			let Ok((events, is_finished)) = result else {
				o.display(MessageContents::Error(
					"Failed to poll remote server for output".into(),
				));
				continue;
			};

			self.apply_events(events, o);
			if is_finished {
				break;
			}
			tokio::time::sleep(Duration::from_millis(150)).await;
		}
	}

	/// Applies a list of output events to the given NitroOutput
	pub fn apply_events(&mut self, events: Vec<OutputEvent>, o: &mut impl NitroOutput) {
		for event in events {
			match event {
				OutputEvent::Message(message) => {
					o.display_message(message);
				}
				OutputEvent::StartProcess => o.start_process(),
				OutputEvent::EndProcess => o.end_process(),
				OutputEvent::StartSection => o.start_section(),
				OutputEvent::EndSection => o.end_section(),
				OutputEvent::PromptYesNo(message) => {
					let _ = o.prompt_yes_no(true, message);
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_full_job() {
		let mut output = RemoteOutput::new(&Paths::new_no_create().unwrap());
		output.new_job();
		let job_id = output.job_id().unwrap();

		output.display_text("Test message".into(), MessageLevel::Important);
		output.finish_job(job_id);

		let job = output.get_job(job_id).unwrap();
		assert_eq!(job.events.len(), 1);
		assert!(job.is_finished);
	}
}
