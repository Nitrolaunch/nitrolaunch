use std::{
	borrow::Cow,
	future::Future,
	marker::PhantomPinned,
	pin::Pin,
	task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pub trait TryReadExt: AsyncRead {
	/// Tries to read some bytes from a buffer, returning none if no bytes were immediately read
	fn try_read<'a>(&'a mut self, buf: &'a mut [u8]) -> TryRead<'a, Self> {
		TryRead {
			reader: self,
			buf,
			_pin: PhantomPinned,
		}
	}
}

impl<R: AsyncRead> TryReadExt for R {}

pin_project! {
	#[derive(Debug)]
	#[must_use = "futures do nothing unless you `.await` or poll them"]
	pub struct TryRead<'a, R: ?Sized> {
		reader: &'a mut R,
		buf: &'a mut [u8],
		// Make this future `!Unpin` for compatibility with async trait methods.
		#[pin]
		_pin: PhantomPinned,
	}
}

impl<R> Future for TryRead<'_, R>
where
	R: AsyncRead + Unpin + ?Sized,
{
	type Output = std::io::Result<Option<usize>>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<Option<usize>>> {
		let me = self.project();
		let mut buf = ReadBuf::new(me.buf);
		let result = Pin::new(me.reader).poll_read(cx, &mut buf);
		match result {
			Poll::Pending => Poll::Ready(Ok(None)),
			Poll::Ready(result) => {
				result?;
				Poll::Ready(Ok(Some(buf.filled().len())))
			}
		}
	}
}

/// Tries to read buffered lines from an AsyncRead
pub struct TryLineReader<R> {
	reader: R,
	/// Buffer for the current line
	current_line: String,
	/// Buffer for reading
	read_buf: [u8; BUF_SIZE],
}

impl<R: TryReadExt + Unpin> TryLineReader<R> {
	pub fn new(reader: R) -> Self {
		Self {
			reader,
			current_line: String::new(),
			read_buf: [0u8; BUF_SIZE],
		}
	}

	/// Reads lines from the reader. Returns None on EoF
	pub async fn lines<'a>(&'a mut self) -> anyhow::Result<Option<Vec<Cow<'a, str>>>> {
		let result_len = self.reader.try_read(&mut self.read_buf).await?;
		let Some(result_len) = result_len else {
			return Ok(Some(Vec::new()));
		};

		// EoF
		if result_len == 0 {
			return Ok(None);
		}

		// Split the read bytes into lines, combining the first line with the contents of the line buf and putting the last partial line into the line buf
		let mut out = Vec::new();
		let read_string = std::str::from_utf8(&self.read_buf)?;

		// No newlines yet, just add to the current line
		if !read_string.contains("\n") {
			self.current_line.push_str(read_string);
			return Ok(Some(Vec::new()));
		}

		let mut lines = read_string.lines();
		let first_line = lines
			.next()
			.expect("Split containing the pattern should be len >= 2");
		let mut lines = lines.rev();
		let last_line = lines
			.next()
			.expect("Split containing the pattern should be len >= 2");
		let lines = lines.rev();

		// Combine the first line with the current line buf
		let mut first_line2 = self.current_line.clone();
		first_line2.push_str(first_line);
		out.push(Cow::Owned(first_line2));
		self.current_line.clear();

		// Deal with all the lines in the middle
		out.extend(lines.map(Cow::Borrowed));

		// Add the last line to the line buffer
		self.current_line.push_str(last_line);

		Ok(Some(out))
	}
}

const BUF_SIZE: usize = 16384;
