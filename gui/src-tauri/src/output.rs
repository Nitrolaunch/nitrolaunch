use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context;
use nitrolaunch::shared::id::InstanceID;
use nitrolaunch::shared::lang::translate::TranslationKey;
use nitrolaunch::shared::output::{Message, MessageContents, MessageLevel, NitroOutput};
use nitrolaunch::shared::pkg::{ArcPkgReq, ResolutionError};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc::Sender, Mutex};

/// Response to a prompt in the frontend, shared with a mutex
pub type PromptResponse = Arc<Mutex<Option<String>>>;

pub struct LauncherOutput {
	inner: OutputInner,
	/// The task that this output is running
	task: Option<String>,
	/// The instance launch associated with this specific output
	instance: Option<InstanceID>,
}

impl LauncherOutput {
	pub fn new(inner: &OutputInner) -> Self {
		Self {
			inner: inner.clone(),
			task: None,
			instance: None,
		}
	}

	pub fn set_task(&mut self, task: &str) {
		let _ = self.inner.app.emit_all("nitro_output_create_task", task);
		self.task = Some(task.to_string());
	}

	pub fn set_instance(&mut self, instance: InstanceID) {
		self.instance = Some(instance);
	}

	pub fn finish_task(&self) {
		if let Some(task) = &self.task {
			let _ = self.inner.app.emit_all("nitro_output_finish_task", task);
		}
	}
}

#[async_trait::async_trait]
impl NitroOutput for LauncherOutput {
	fn display_text(&mut self, text: String, _level: MessageLevel) {
		self.disp(text);
	}

	fn display_message(&mut self, message: Message) {
		let logger = self.inner.logger.clone();
		let message2 = message.clone();
		tokio::task::spawn(async move {
			let _ = logger.send(message2).await;
		});

		if !message.level.at_least(&MessageLevel::Extra) {
			return;
		}

		match message.contents {
			MessageContents::Associated(assoc, msg) => match *assoc {
				MessageContents::Progress { current, total } => {
					let _ = self.inner.app.emit_all(
						"nitro_output_progress",
						AssociatedProgressEvent {
							current,
							total,
							message: msg.default_format(),
							task: self.task.clone(),
						},
					);
				}
				_ => self.disp(format!(
					"({}) {}",
					assoc.default_format(),
					msg.default_format()
				)),
			},
			MessageContents::Header(text) => {
				let _ = self.inner.app.emit_all(
					"nitro_output_message",
					MessageEvent {
						message: text,
						ty: MessageType::Header,
						task: self.task.clone(),
					},
				);
			}
			MessageContents::StartProcess(text) => {
				let _ = self.inner.app.emit_all(
					"nitro_output_message",
					MessageEvent {
						message: text,
						ty: MessageType::StartProcess,
						task: self.task.clone(),
					},
				);
			}
			MessageContents::Warning(text) => {
				let _ = self.inner.app.emit_all(
					"nitro_output_message",
					MessageEvent {
						message: text,
						ty: MessageType::Warning,
						task: self.task.clone(),
					},
				);
			}
			MessageContents::Error(text) => {
				eprintln!("Error: {text}");
				let _ = self.inner.app.emit_all(
					"nitro_output_message",
					MessageEvent {
						message: text,
						ty: MessageType::Error,
						task: self.task.clone(),
					},
				);
			}
			msg => self.disp(msg.default_format()),
		}
	}

	async fn prompt_special_user_passkey(
		&mut self,
		message: MessageContents,
		user_id: &str,
	) -> anyhow::Result<String> {
		{
			let passkeys = self.inner.passkeys.lock().await;
			if let Some(existing) = passkeys.get(user_id) {
				return Ok(existing.clone());
			}
		}

		let result = self.prompt_password(message).await?;
		let mut passkeys = self.inner.passkeys.lock().await;
		passkeys.insert(user_id.into(), result.clone());
		Ok(result)
	}

	async fn prompt_password(&mut self, message: MessageContents) -> anyhow::Result<String> {
		println!("Starting password prompt");
		self.inner
			.app
			.emit_all("nitro_display_password_prompt", message.default_format())
			.context("Failed to display password prompt to user")?;

		// Block this thread, checking every interval if the prompt has been filled
		let result = loop {
			if let Some(answer) = self.inner.password_prompt.lock().await.take() {
				break answer;
			}
			tokio::time::sleep(Duration::from_millis(50)).await;
		};

		Ok(result)
	}

	async fn prompt_new_password(&mut self, message: MessageContents) -> anyhow::Result<String> {
		self.prompt_password(message).await
	}

	fn display_special_ms_auth(&mut self, url: &str, code: &str) {
		self.display_text("Showing auth info".into(), MessageLevel::Important);
		let _ = self.inner.app.emit_all(
			"nitro_display_auth_info",
			AuthDisplayEvent {
				url: url.to_owned(),
				device_code: code.to_owned(),
			},
		);
	}

	fn display_special_resolution_error(&mut self, error: ResolutionError, instance_id: &str) {
		eprintln!("Resolution error: {error:?}");
		let error = SerializableResolutionError::from_err(error);

		let payload = ResolutionErrorEvent {
			error,
			instance: instance_id.to_string(),
		};

		self.inner.app.trigger_global(
			"nitro_display_resolution_error",
			Some(serde_json::to_string(&payload).unwrap()),
		);
		let _ = self
			.inner
			.app
			.emit_all("nitro_display_resolution_error", payload);
	}

	fn translate(&self, key: TranslationKey) -> &str {
		// Emit an event for certain keys as they notify us of progress in the launch
		if let TranslationKey::AuthenticationSuccessful = key {
			let _ = self.inner.app.emit_all("nitro_close_auth_info", ());
		}

		key.get_default()
	}

	fn start_process(&mut self) {
		let _ = self
			.inner
			.app
			.emit_all("nitro_output_start_process", &self.task);
	}

	fn end_process(&mut self) {
		let _ = self
			.inner
			.app
			.emit_all("nitro_output_end_process", &self.task);
	}

	fn start_section(&mut self) {
		let _ = self
			.inner
			.app
			.emit_all("nitro_output_start_section", &self.task);
	}

	fn end_section(&mut self) {
		let _ = self
			.inner
			.app
			.emit_all("nitro_output_end_section", &self.task);
	}
}

impl LauncherOutput {
	fn disp(&mut self, text: String) {
		println!("{text}");
		let _ = self.inner.app.emit_all(
			"nitro_output_message",
			MessageEvent {
				message: text,
				ty: MessageType::Simple,
				task: self.task.clone(),
			},
		);
	}
}

impl Drop for LauncherOutput {
	fn drop(&mut self) {
		self.finish_task();
	}
}

#[derive(Clone)]
pub struct OutputInner {
	pub app: Arc<AppHandle>,
	pub password_prompt: PromptResponse,
	pub passkeys: Arc<Mutex<HashMap<String, String>>>,
	pub logger: Sender<Message>,
}

/// Event for a simple text message
#[derive(Clone, Serialize)]
pub struct MessageEvent {
	pub message: String,
	#[serde(rename = "type")]
	pub ty: MessageType,
	pub task: Option<String>,
}

/// Event for an associated progressbar
#[derive(Clone, Serialize)]
pub struct AssociatedProgressEvent {
	pub current: u32,
	pub total: u32,
	pub message: String,
	pub task: Option<String>,
}

/// Event for the auth display
#[derive(Clone, Serialize)]
pub struct AuthDisplayEvent {
	url: String,
	device_code: String,
}

/// Event for a yes-no prompt
#[derive(Clone, Serialize)]
pub struct YesNoPromptEvent {
	default: bool,
	message: String,
}

/// Event for a package resolution error
#[derive(Clone, Serialize, Deserialize)]
pub struct ResolutionErrorEvent {
	pub error: SerializableResolutionError,
	pub instance: String,
}

/// A serializable ResolutionError
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "data")]
pub enum SerializableResolutionError {
	PackageContext(ArcPkgReq, Box<SerializableResolutionError>),
	FailedToPreload(String),
	FailedToGetProperties(ArcPkgReq, String),
	NoValidVersionsFound(ArcPkgReq),
	ExtensionNotFulfilled(Option<ArcPkgReq>, ArcPkgReq),
	ExplicitRequireNotFulfilled(ArcPkgReq, ArcPkgReq),
	IncompatiblePackage(ArcPkgReq, Vec<Arc<str>>),
	FailedToEvaluate(ArcPkgReq, String),
	Misc(String),
}

impl SerializableResolutionError {
	pub fn from_err(err: ResolutionError) -> Self {
		match err {
			ResolutionError::PackageContext(req, resolution_error) => {
				SerializableResolutionError::PackageContext(
					req,
					Box::new(SerializableResolutionError::from_err(*resolution_error)),
				)
			}
			ResolutionError::FailedToPreload(error) => {
				SerializableResolutionError::FailedToPreload(error.to_string())
			}
			ResolutionError::FailedToGetProperties(req, error) => {
				SerializableResolutionError::FailedToGetProperties(req, format!("{error:?}"))
			}
			ResolutionError::NoValidVersionsFound(req) => {
				SerializableResolutionError::NoValidVersionsFound(req)
			}
			ResolutionError::ExtensionNotFulfilled(req1, req2) => {
				SerializableResolutionError::ExtensionNotFulfilled(req1, req2)
			}
			ResolutionError::ExplicitRequireNotFulfilled(req1, req2) => {
				SerializableResolutionError::ExplicitRequireNotFulfilled(req1, req2)
			}
			ResolutionError::IncompatiblePackage(req, items) => {
				SerializableResolutionError::IncompatiblePackage(req, items)
			}
			ResolutionError::FailedToEvaluate(req, error) => {
				SerializableResolutionError::FailedToEvaluate(req, format!("{error:?}"))
			}
			ResolutionError::Misc(error) => SerializableResolutionError::Misc(format!("{error:?}")),
		}
	}
}

#[derive(Clone, Serialize, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
	Simple,
	Header,
	StartProcess,
	Warning,
	Error,
}
