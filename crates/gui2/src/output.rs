use std::{collections::HashMap, sync::Arc, time::Duration};

use nitrolaunch::shared::{
	id::InstanceID,
	lang::translate::TranslationKey,
	output::{Message, MessageContents, MessageLevel, NitroOutput},
	pkg::{PackageDiff, ResolutionError},
};
use tokio::sync::{Mutex, broadcast, mpsc};

use crate::state::BackEvent;

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
		let _ = self
			.inner
			.event_tx
			.send(BackEvent::OutputStartTask(task.into()));
		self.task = Some(task.to_string());
	}

	pub fn set_instance(&mut self, instance: InstanceID) {
		self.instance = Some(instance);
	}

	pub fn finish_task(&mut self) {
		if let Some(task) = &self.task {
			let _ = self
				.inner
				.event_tx
				.send(BackEvent::OutputEndTask(task.clone()));
			self.task = None;
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

		let _ = self.inner.event_tx.send(BackEvent::OutputMessage {
			message: message.contents,
			task: self.task.clone(),
		});
	}

	async fn prompt_yes_no(
		&mut self,
		default: bool,
		message: MessageContents,
	) -> anyhow::Result<bool> {
		let _ = default;
		self.inner.yes_no_prompt.lock().await.take();
		let _ = self.inner.event_tx.send(BackEvent::ShowYesNoPrompt {
			message: message.default_format(),
		});

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

	async fn prompt_password(&mut self, _: MessageContents) -> anyhow::Result<String> {
		println!("Starting password prompt");
		let _ = self.inner.event_tx.send(BackEvent::ShowPasskeyPrompt);

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

		let _ = self
			.inner
			.event_tx
			.send(BackEvent::ShowPackageDiffsPrompt { diffs });

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
		let _ = self.inner.event_tx.send(BackEvent::ShowAuthPrompt {
			url: url.into(),
			device_code: code.into(),
		});
	}

	fn display_special_resolution_error(&mut self, error: ResolutionError, instance_id: &str) {
		eprintln!("Package resolution error: {error}");
		let _ = self.inner.event_tx.send(BackEvent::OutputResolutionError {
			error: Arc::new(error),
			instance_id: instance_id.to_string(),
		});
	}

	fn translate(&self, key: TranslationKey) -> &str {
		// Emit an event for certain keys as they notify us of progress in the launch
		if let TranslationKey::AuthenticationSuccessful = key {
			let _ = self.inner.event_tx.send(BackEvent::CloseAuthPrompt);
		}

		key.get_default()
	}

	fn end_process(&mut self) {
		let _ = self
			.inner
			.event_tx
			.send(BackEvent::OutputEndProcess(self.task.clone()));
	}

	fn end_section(&mut self) {
		let _ = self
			.inner
			.event_tx
			.send(BackEvent::OutputEndSection(self.task.clone()));
	}

	fn get_lesser_copy(&self) -> Box<dyn NitroOutput + Sync> {
		Box::new(Self::new(&self.inner))
	}
}

impl LauncherOutput {
	fn disp(&mut self, text: String) {
		println!("{text}");
		let _ = self.inner.event_tx.send(BackEvent::OutputMessage {
			message: MessageContents::Simple(text),
			task: self.task.clone(),
		});
	}
}

impl Drop for LauncherOutput {
	fn drop(&mut self) {
		self.finish_task();
	}
}

#[derive(Clone)]
pub struct OutputInner {
	pub event_tx: broadcast::Sender<BackEvent>,
	pub password_prompt: PromptResponse,
	pub yes_no_prompt: YesNoPromptResponse,
	pub passkeys: Arc<Mutex<HashMap<String, String>>>,
	pub logger: mpsc::Sender<Message>,
}
