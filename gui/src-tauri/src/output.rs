use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context;
use nitrolaunch::shared::id::InstanceID;
use nitrolaunch::shared::lang::translate::TranslationKey;
use nitrolaunch::shared::output::{Message, MessageContents, MessageLevel, NitroOutput};
use nitrolaunch::shared::pkg::{ArcPkgReq, PackageDiff, ResolutionError};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc::Sender, Mutex};

/// Response to a prompt in the frontend, shared with a mutex
pub type PromptResponse = Arc<Mutex<Option<String>>>;
/// Response to a yes/no prompt in the frontend, shared with a mutex
pub type YesNoPromptResponse = Arc<Mutex<Option<bool>>>;

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
		let _ = self.inner.app.emit("nitro_output_create_task", task);
		self.task = Some(task.to_string());
	}

	pub fn set_instance(&mut self, instance: InstanceID) {
		self.instance = Some(instance);
	}

	pub fn finish_task(&self) {
		if let Some(task) = &self.task {
			let _ = self.inner.app.emit("nitro_output_finish_task", task);
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

		if message.level < MessageLevel::Important {
			return;
		}

		match message.contents {
			MessageContents::Associated(assoc, msg) => match *assoc {
				MessageContents::Progress { current, total } => {
					let _ = self.inner.app.emit(
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
				let _ = self.inner.app.emit(
					"nitro_output_message",
					MessageEvent {
						message: text,
						ty: MessageType::Header,
						task: self.task.clone(),
					},
				);
			}
			MessageContents::StartProcess(text) => {
				let _ = self.inner.app.emit(
					"nitro_output_message",
					MessageEvent {
						message: text,
						ty: MessageType::StartProcess,
						task: self.task.clone(),
					},
				);
			}
			MessageContents::Warning(text) => {
				let _ = self.inner.app.emit(
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
				let _ = self.inner.app.emit(
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

	async fn prompt_yes_no(
		&mut self,
		default: bool,
		message: MessageContents,
	) -> anyhow::Result<bool> {
		let _ = default;
		self.inner.yes_no_prompt.lock().await.take();

		self.inner
			.app
			.emit("nitro_display_yes_no_prompt", message.default_format())
			.context("Failed to display yes/no prompt to user")?;

		// Block this thread, checking every interval if the prompt has been filled
		let result = loop {
			if let Some(answer) = self.inner.yes_no_prompt.lock().await.take() {
				break answer;
			}
			tokio::time::sleep(Duration::from_millis(50)).await;
		};

		Ok(result)
	}

	async fn prompt_special_account_passkey(
		&mut self,
		message: MessageContents,
		account_id: &str,
	) -> anyhow::Result<String> {
		{
			let passkeys = self.inner.passkeys.lock().await;
			if let Some(existing) = passkeys.get(account_id) {
				return Ok(existing.clone());
			}
		}

		let result = self.prompt_password(message).await?;
		let mut passkeys = self.inner.passkeys.lock().await;
		passkeys.insert(account_id.into(), result.clone());
		Ok(result)
	}

	async fn prompt_password(&mut self, message: MessageContents) -> anyhow::Result<String> {
		println!("Starting password prompt");
		self.inner
			.app
			.emit("nitro_display_password_prompt", message.default_format())
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

	async fn prompt_special_package_diffs(
		&mut self,
		diffs: Vec<PackageDiff>,
	) -> anyhow::Result<bool> {
		self.inner.yes_no_prompt.lock().await.take();

		let diffs: Vec<_> = diffs
			.into_iter()
			.map(SerializablePackageDiff::from_diff)
			.collect();

		self.inner
			.app
			.emit("nitro_display_package_diffs_prompt", &diffs)
			.context("Failed to display package diff prompt to user")?;

		// Block this thread, checking every interval if the prompt has been filled
		let result = loop {
			if let Some(answer) = self.inner.yes_no_prompt.lock().await.take() {
				break answer;
			}
			tokio::time::sleep(Duration::from_millis(50)).await;
		};

		Ok(result)
	}

	fn display_special_ms_auth(&mut self, url: &str, code: &str) {
		self.display_text("Showing auth info".into(), MessageLevel::Important);
		let _ = self.inner.app.emit(
			"nitro_display_auth_info",
			AuthDisplayEvent {
				url: url.to_owned(),
				device_code: code.to_owned(),
			},
		);
	}

	fn display_special_resolution_error(&mut self, error: ResolutionError, instance_id: &str) {
		eprintln!("Resolution error: {error}");
		let error = SerializableResolutionError::from_err(error);

		let payload = ResolutionErrorEvent {
			error,
			instance: instance_id.to_string(),
		};

		let _ = self
			.inner
			.app
			.emit("nitro_display_resolution_error", payload);
	}

	fn translate(&self, key: TranslationKey) -> &str {
		// Emit an event for certain keys as they notify us of progress in the launch
		if let TranslationKey::AuthenticationSuccessful = key {
			let _ = self.inner.app.emit("nitro_close_auth_info", ());
		}

		key.get_default()
	}

	fn start_process(&mut self) {
		let _ = self
			.inner
			.app
			.emit("nitro_output_start_process", &self.task);
	}

	fn end_process(&mut self) {
		let _ = self.inner.app.emit("nitro_output_end_process", &self.task);
	}

	fn start_section(&mut self) {
		let _ = self
			.inner
			.app
			.emit("nitro_output_start_section", &self.task);
	}

	fn end_section(&mut self) {
		let _ = self.inner.app.emit("nitro_output_end_section", &self.task);
	}

	fn get_lesser_copy(&self) -> Box<dyn NitroOutput + Sync> {
		Box::new(Self::new(&self.inner))
	}
}

impl LauncherOutput {
	fn disp(&mut self, text: String) {
		println!("{text}");
		let _ = self.inner.app.emit(
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
	pub yes_no_prompt: YesNoPromptResponse,
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
	NoValidVersionsFound(ArcPkgReq, Vec<String>),
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
			ResolutionError::NoValidVersionsFound(req, constraints) => {
				SerializableResolutionError::NoValidVersionsFound(
					req,
					constraints.into_iter().map(|x| x.to_string()).collect(),
				)
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

/// A change to an installed package, used for user display
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "data")]
pub enum SerializablePackageDiff {
	/// A new package was added
	Added(String),
	/// A large number of packages were added
	ManyAdded(u16),
	/// An existing package was removed
	Removed(String),
	/// A large number of packages were removed
	ManyRemoved(u16),
	/// An existing package had it's version changed. Contains the old and new version
	VersionChanged(String, String, String),
}

impl SerializablePackageDiff {
	pub fn from_diff(diff: PackageDiff) -> Self {
		match diff {
			PackageDiff::Added(pkg) => Self::Added(pkg.to_string()),
			PackageDiff::Removed(pkg) => Self::Removed(pkg.to_string()),
			PackageDiff::VersionChanged(pkg, old_version, new_version) => {
				Self::VersionChanged(pkg.to_string(), old_version, new_version)
			}
			PackageDiff::ManyAdded(count) => Self::ManyAdded(count),
			PackageDiff::ManyRemoved(count) => Self::ManyRemoved(count),
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
