use std::{hash::Hash, marker::PhantomData, pin::Pin};

use freya::query::{Query, QueryCapability};

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
