use std::time::SystemTime;

/// Profiler that marks times to complete certain parts of code
pub struct Profiler {
	time: SystemTime,
}

impl Profiler {
	/// Create a new profiler that starts profiling immediately
	pub fn new() -> Self {
		Self {
			time: SystemTime::now(),
		}
	}

	/// Marks a new time event and restarts for the next event
	pub fn time(&mut self, title: &str) {
		let now = SystemTime::now();
		let elapsed = now.duration_since(self.time).unwrap();
		println!("{title}: {elapsed:?}");
		self.time = now;
	}
}
