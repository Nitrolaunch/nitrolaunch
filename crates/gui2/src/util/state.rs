use std::sync::Arc;

use gpui::{AsyncApp, Context, Entity};
use tokio::sync::{Mutex, MutexGuard};

pub fn setter<T: 'static, V: 'static, F: Fn(&mut V, T) + Clone>(
	entity: Entity<V>,
	f: F,
) -> impl Fn(T, &mut Context<V>) + Clone {
	move |value, cx| {
		entity.update(cx, |this, cx| {
			f(this, value);
			cx.notify();
		});
	}
}

/// Easily checkable event trigger
pub struct Trigger(bool);

impl Trigger {
	pub fn new() -> Self {
		Self(false)
	}

	pub fn trigger(&mut self) {
		self.0 = true;
	}

	pub fn check(&mut self) -> bool {
		if self.0 {
			self.0 = false;
			true
		} else {
			false
		}
	}
}

/// Asynchronously loaded data on a component
pub struct Resource<T> {
	state: Arc<Mutex<ResourceState<T>>>,
}

impl<T: 'static> Resource<T> {
	pub fn new() -> Self {
		let state = Arc::new(Mutex::new(ResourceState::Loading));
		Self { state }
	}

	pub fn fetch<V: 'static, F>(&self, cx: &mut Context<V>, f: F)
	where
		F: (AsyncFn(&mut AsyncApp) -> anyhow::Result<T>) + 'static,
	{
		let state = self.state.clone();
		cx.spawn(async move |e, cx| {
			let result = f(cx).await;
			*state.lock().await = match result {
				Ok(result) => ResourceState::Loaded(result),
				Err(e) => ResourceState::Err(e),
			};

			let _ = e.update(cx, |_, cx| {
				cx.notify();
			});
		})
		.detach();
	}

	pub fn state<'a>(&'a self) -> Option<MutexGuard<'a, ResourceState<T>>> {
		self.state.try_lock().ok()
	}
}

/// State for a loading resource
pub enum ResourceState<T> {
	Loading,
	Loaded(T),
	Err(anyhow::Error),
}
