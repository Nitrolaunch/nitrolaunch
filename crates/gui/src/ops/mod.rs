use std::{hash::Hash, marker::PhantomData, pin::Pin};

use anyhow::anyhow;
use freya::query::{Captured, MutationCapability, QueriesStorage, Query, QueryCapability};
use nitrolaunch::shared::output::MessageContents;

use crate::state::{BackEvent, BackState};

pub mod account;
pub mod instance;
pub mod launch;
pub mod misc;
pub mod packages;
pub mod plugin_results;
pub mod plugins;
pub mod settings;
pub mod task;
pub mod transfer;
pub mod versions;

#[macro_export]
macro_rules! simple_query {
	(
		name = $name:ident,
		ok = $ok:ty,
		err = $err:ty,
		keys = $keys:ty,
		$($run:tt)*
	) => {
		#[derive(Clone, PartialEq, Eq, Hash)]
		pub struct $name {
			back_state: freya::query::Captured<$crate::state::BackState>,
		}

		impl $name {
			pub fn new(back_state: $crate::state::BackState) -> Self {
				Self {
					back_state: freya::query::Captured(back_state),
				}
			}
		}

		impl freya::query::QueryCapability for $name {
			type Ok = $ok;
			type Err = $err;
			type Keys = $keys;

			$($run)*
		}
	};
}

#[macro_export]
macro_rules! simple_mutation {
	(
		name = $name:ident,
		ok = $ok:ty,
		err = $err:ty,
		keys = $keys:ty,
		$($run:tt)*
	) => {
		#[derive(Clone, PartialEq, Eq, Hash)]
		pub struct $name {
			back_state: freya::query::Captured<$crate::state::BackState>,
		}

		impl $name {
			pub fn new(back_state: $crate::state::BackState) -> Self {
				Self {
					back_state: freya::query::Captured(back_state),
				}
			}
		}

		impl freya::query::MutationCapability for $name {
			type Ok = $ok;
			type Err = $err;
			type Keys = $keys;

			$($run)*
		}
	};
}

/// Utility function to spawn for queries with a flattened error type and error handling
pub async fn query_spawn<F, T>(back_state: BackState, f: F) -> anyhow::Result<T>
where
	F: Future + Send + 'static,
	F::Output: AnyhowResult<T> + Send + 'static,
{
	let task = tokio::spawn(f);
	let result = task.await;
	match result {
		Ok(result) => {
			let result = result.into_result();
			if let Err(e) = &result {
				back_state.log(MessageContents::Error(format!(
					"Query or mutation failed: {e:?}"
				)));
			}

			result
		}
		Err(e) => Err(anyhow!("Failed to join: {e}")),
	}
}

pub trait AnyhowResult<T> {
	fn into_result(self) -> anyhow::Result<T>;
}

impl<T> AnyhowResult<T> for anyhow::Result<T> {
	fn into_result(self) -> anyhow::Result<T> {
		self
	}
}

pub trait AnyhowError {
	fn as_err(&self) -> &anyhow::Error;
}

impl AnyhowError for anyhow::Error {
	fn as_err(&self) -> &anyhow::Error {
		self
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConditionalQuery<Q: QueryCapability> {
	_p: PhantomData<Q>,
}

impl<Q: QueryCapability> ConditionalQuery<Q> {
	pub fn new(query: Q, enable: bool, k: impl FnOnce() -> Q::Keys) -> Query<Self> {
		if enable {
			let keys = k();
			Query::new(
				ConditionalKeys::Enabled(query, keys),
				Self { _p: PhantomData },
			)
		} else {
			Query::new(ConditionalKeys::Disabled, Self { _p: PhantomData })
		}
	}
}

impl<Q: QueryCapability> QueryCapability for ConditionalQuery<Q> {
	type Ok = Q::Ok;
	type Err = Q::Err;
	type Keys = ConditionalKeys<Q, Q::Keys>;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		let keys = keys.clone();
		async move {
			match keys {
				ConditionalKeys::Enabled(query, keys) => query.run(&keys).await,
				_ => std::future::pending().await,
			}
		}
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ConditionalKeys<Q, K> {
	Disabled,
	Enabled(Q, K),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ToastedQuery<Q: QueryCapability> {
	query: Q,
	back_state: Captured<BackState>,
	success_message: Option<String>,
	error_message: String,
	_p: PhantomData<Q>,
}

impl<Q: QueryCapability> QueryCapability for ToastedQuery<Q>
where
	Q::Err: AnyhowError,
{
	type Ok = Q::Ok;
	type Err = Q::Err;
	type Keys = Q::Keys;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		async move {
			let result = self.query.run(keys).await;

			match &result {
				Ok(..) => {
					if let Some(success_message) = &self.success_message {
						let _ = self
							.back_state
							.event_tx
							.send(BackEvent::SuccessToast(success_message.clone()));
					}
				}
				Err(e) => {
					let _ = self.back_state.event_tx.send(BackEvent::ErrorToast(
						self.error_message.clone(),
						Some(format!("{:?}", e.as_err())),
					));
				}
			}

			result
		}
	}
}

pub trait ToastedQueryExt: QueryCapability {
	fn toast(
		self,
		back_state: &BackState,
		success_message: Option<&str>,
		error_message: &str,
	) -> ToastedQuery<Self>;
}

impl<Q: QueryCapability> ToastedQueryExt for Q
where
	Q::Err: AnyhowError,
{
	fn toast(
		self,
		back_state: &BackState,
		success_message: Option<&str>,
		error_message: &str,
	) -> ToastedQuery<Self> {
		ToastedQuery {
			query: self,
			back_state: Captured(back_state.clone()),
			success_message: success_message.map(|x| x.to_string()),
			error_message: error_message.to_string(),
			_p: PhantomData,
		}
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ToastedMutation<M: MutationCapability> {
	mutation: M,
	back_state: Captured<BackState>,
	success_message: Option<String>,
	error_message: String,
	_p: PhantomData<M>,
}

impl<M: MutationCapability> MutationCapability for ToastedMutation<M>
where
	M::Err: AnyhowError,
{
	type Ok = M::Ok;
	type Err = M::Err;
	type Keys = M::Keys;

	fn run(&self, keys: &Self::Keys) -> impl Future<Output = Result<Self::Ok, Self::Err>> {
		self.mutation.run(keys)
	}

	fn on_settled(
		&self,
		keys: &Self::Keys,
		result: &Result<Self::Ok, Self::Err>,
	) -> impl Future<Output = ()> {
		match result {
			Ok(..) => {
				if let Some(success_message) = &self.success_message {
					let _ = self
						.back_state
						.event_tx
						.send(BackEvent::SuccessToast(success_message.clone()));
				}
			}
			Err(e) => {
				let _ = self.back_state.event_tx.send(BackEvent::ErrorToast(
					self.error_message.clone(),
					Some(format!("{:?}", e.as_err())),
				));
			}
		}

		self.mutation.on_settled(keys, result)
	}
}

pub trait ToastedMutationExt: MutationCapability {
	fn toast(
		self,
		back_state: &BackState,
		success_message: Option<&str>,
		error_message: &str,
	) -> ToastedMutation<Self>;
}

impl<M: MutationCapability> ToastedMutationExt for M
where
	M::Err: AnyhowError,
{
	fn toast(
		self,
		back_state: &BackState,
		success_message: Option<&str>,
		error_message: &str,
	) -> ToastedMutation<Self> {
		ToastedMutation {
			mutation: self,
			back_state: Captured(back_state.clone()),
			success_message: success_message.map(|x| x.to_string()),
			error_message: error_message.to_string(),
			_p: PhantomData,
		}
	}
}

/// Invalidate that handles ToastedQuery and ConditionalQuery
pub async fn invalidate_all<Q: QueryCapability>()
where
	Q::Err: AnyhowError,
{
	QueriesStorage::<Q>::try_invalidate_all().await;
	QueriesStorage::<ToastedQuery<Q>>::try_invalidate_all().await;
	QueriesStorage::<ConditionalQuery<Q>>::try_invalidate_all().await;
}

/// Invalidate matching that handles ToastedQuery and ConditionalQuery
pub async fn invalidate_matching<Q: QueryCapability>(keys: Q::Keys)
where
	Q::Err: AnyhowError,
{
	QueriesStorage::<Q>::try_invalidate_matching(keys.clone()).await;
	QueriesStorage::<ToastedQuery<Q>>::try_invalidate_matching(keys.clone()).await;
	// Too hard to check this
	QueriesStorage::<ConditionalQuery<Q>>::try_invalidate_all().await;
}

/// Utility to get around some Rust incapabilities, forcing a future to be send
pub struct MakeSend<F: Future>(Pin<Box<F>>);

unsafe impl<F: Future> Send for MakeSend<F> {}

impl<F: Future> Future for MakeSend<F> {
	type Output = F::Output;

	fn poll(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		F::poll(self.0.as_mut(), cx)
	}
}

impl<F: Future> MakeSend<F> {
	/// SAFETY: None. The future better actually be send!
	pub unsafe fn new(f: F) -> Self {
		Self(Box::pin(f))
	}
}
