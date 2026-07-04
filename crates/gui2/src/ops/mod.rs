use std::{hash::Hash, marker::PhantomData, pin::Pin};

use freya::query::{Captured, MutationCapability, Query, QueryCapability};

use crate::{
	state::{BackEvent, BackState},
	util::AnyhowError,
};

pub mod instance;
pub mod launch;
pub mod packages;
pub mod plugin_results;
pub mod task;
pub mod versions;

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
						Some(e.as_err().to_string()),
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
		_keys: &Self::Keys,
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
					Some(e.as_err().to_string()),
				));
			}
		}

		std::future::ready(())
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
