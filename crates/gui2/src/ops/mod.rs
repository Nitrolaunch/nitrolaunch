use std::pin::Pin;

pub mod instance;
pub mod launch;
pub mod task;

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
