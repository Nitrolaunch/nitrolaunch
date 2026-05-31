use std::{
	cell::{Ref, RefCell, RefMut},
	rc::Rc,
	time::SystemTime,
};

use anyhow::anyhow;
use freya::prelude::Color;

pub mod assets;

#[macro_export]
macro_rules! clone_into {
	([ $($orig:tt as $var:ident),* $(,)? ], $f:expr) => {
		{
			$(
				let $var = $orig.clone();
			)*

			$f
		}
	};

	([ $($var:expr),* $(,)? ], $f:expr) => {
		{
			$(
				let $var = $var.clone();
			)*

			$f
		}
	};
}

/// Utility function to spawn for queries with a flattened error type
pub async fn query_spawn<F, T>(f: F) -> anyhow::Result<T>
where
	F: Future + Send + 'static,
	F::Output: AnyhowResult<T> + Send + 'static,
{
	let task = tokio::spawn(f);
	let result = task.await;
	match result {
		Ok(result) => result.into_result(),
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

#[derive(Clone)]
pub struct Shared<T> {
	inner: Rc<RefCell<T>>,
}

impl<T> Shared<T> {
	pub fn new(value: T) -> Self {
		Self {
			inner: Rc::new(RefCell::new(value)),
		}
	}

	pub fn read(&self) -> Ref<'_, T> {
		self.inner.borrow()
	}

	pub fn write(&self) -> RefMut<'_, T> {
		self.inner.borrow_mut()
	}
}

#[derive(Clone)]
pub struct NotEq<T>(pub T);

impl<T> PartialEq for NotEq<T> {
	fn eq(&self, _: &Self) -> bool {
		false
	}
}

/// Used for debugging boxes
#[allow(dead_code)]
pub fn random_color() -> Color {
	let time = SystemTime::now().elapsed().unwrap_or_default().as_nanos();
	let color = (time % 10000 * 100 % 255) as u8;

	(color, 0, 0).into()
}
