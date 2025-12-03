/// Utilities for working with hashes and checksums
pub mod hash;

use std::mem::ManuallyDrop;

use rand::Rng;

/// Selects a random set of n elements from a list. The return slice will not necessarily be of n length
pub fn select_random_n_items_from_list<T>(list: &[T], n: usize) -> Vec<&T> {
	let mut indices: Vec<usize> = (0..list.len()).collect();
	let mut rng = rand::thread_rng();
	let mut chosen = Vec::new();
	for _ in 0..n {
		if indices.is_empty() {
			break;
		}

		let index = rng.gen_range(0..indices.len());
		let index = indices.remove(index);
		chosen.push(&list[index]);
	}

	chosen
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
	fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
}

/// Creates a tokio file from the stdin of this process
pub fn get_stdin_file() -> ManuallyDrop<tokio::fs::File> {
	#[cfg(target_os = "windows")]
	{
		let handle = unsafe { GetStdHandle(0xFFFFFFF6) };
		unsafe {
			ManuallyDrop::new(tokio::fs::File::from_std(std::fs::File::from_raw_handle(
				handle,
			)))
		}
	}
	#[cfg(target_family = "unix")]
	{
		use std::os::fd::FromRawFd;
		unsafe { ManuallyDrop::new(tokio::fs::File::from_raw_fd(0)) }
	}
}
