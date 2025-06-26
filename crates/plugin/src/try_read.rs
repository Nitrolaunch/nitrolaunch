use std::{
	borrow::Cow,
	future::Future,
	marker::PhantomPinned,
	pin::Pin,
	task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

/// Trait to read some or none bytes from an AsyncRead
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
	/// Future returned from the try_read function
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

/// Tries to read buffered lines from an AsyncRead.
/// Note that this will only return lines that end with a newline, so the final text between a newline
/// and EoF will not be returned.
pub struct TryLineReader<R> {
	reader: R,
	/// Buffer for the current line
	current_line: String,
	/// Buffer for reading
	read_buf: Vec<u8>,
}

impl<R: TryReadExt + Unpin> TryLineReader<R> {
	/// Create a new TryLineReader
	pub fn new(reader: R) -> Self {
		Self {
			reader,
			current_line: String::new(),
			read_buf: vec![0; BUF_SIZE],
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
			// Handle the last line
			if !self.current_line.is_empty() {
				let last_line = self.current_line.clone();
				self.current_line.clear();
				return Ok(Some(vec![Cow::Owned(last_line)]));
			}
			return Ok(None);
		}

		// Split the read bytes into lines, combining the first line with the contents of the line buf and putting the last partial line into the line buf
		let read_string = std::str::from_utf8(&self.read_buf[0..result_len])?;

		// No newlines yet, just add to the current line
		if !read_string.contains("\n") {
			self.current_line.push_str(read_string);
			return Ok(Some(Vec::new()));
		} else if read_string.chars().last() == Some('\n')
			&& read_string.chars().filter(|x| *x == '\n').count() == 1
		{
			// Special case with one newline exactly at the end. The split will return only one element.
			let mut first_line = self.current_line.clone();
			first_line.push_str(read_string.trim_end_matches("\r\n").trim_end_matches("\n"));

			self.current_line.clear();
			return Ok(Some(vec![Cow::Owned(first_line)]));
		}

		let mut out = Vec::new();

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

		// Add the last line to the line buffer, only if it wasn't a full line
		if read_string.chars().last() == Some('\n') {
			out.push(Cow::Borrowed(last_line));
		} else {
			self.current_line.push_str(last_line);
		}

		Ok(Some(out))
	}
}

const BUF_SIZE: usize = 32768;

#[cfg(test)]
mod test {
	use std::collections::VecDeque;

	use super::*;

	/// Reader for tests that outputs one of the given outputs every time try_read is called
	struct TestReader {
		outputs: VecDeque<&'static str>,
	}

	impl AsyncRead for TestReader {
		fn poll_read(
			mut self: Pin<&mut Self>,
			cx: &mut Context<'_>,
			buf: &mut ReadBuf<'_>,
		) -> Poll<std::io::Result<()>> {
			let _ = cx;

			let Some(next) = self.outputs.pop_front() else {
				return Poll::Ready(Ok(()));
			};

			buf.put_slice(next.as_bytes());

			Poll::Ready(Ok(()))
		}
	}

	#[tokio::test]
	async fn test_no_lines() {
		test(&[""], &[]).await;
		test(&["foobar", "foobar,", "barfoo"], &[]).await;
	}

	#[tokio::test]
	async fn test_split_lines() {
		test(&["foo", "bar\n", "foobar\n"], &["foobar", "foobar"]).await;
	}

	#[tokio::test]
	async fn test_combined_lines() {
		test(
			&["foo", "bar\nbaz\nbar\n\nbaz", "baz\n"],
			&["foobar", "baz", "bar", "", "bazbaz"],
		)
		.await;
	}

	#[tokio::test]
	async fn test_real_data() {
		test(
			&["%_eyJtZXNzYWdlIjp7ImNvbnRlbnRzIjp7IlN1Y2Nlc3MiOiJGYWJyaWMgZG93bmxvYWRlZCJ9LCJsZXZlbCI6ImltcG9ydGFudCJ9fQ==\n%_ImVuZF9wcm9jZXNzIg==\n%_eyJzZXRfcmVzdWx0Ijp7ImNsYXNzcGF0aF9leHRlbnNpb24iOlsiL2hvbWUvcGFuZ28vLmxvY2FsL3NoYXJlL21jdm0vaW50ZXJuYWwvbGlicmFyaWVzL29yZy9vdzIvYXNtL2FzbS85LjgvYXNtLTkuOC5qYXIiLCIvaG9tZS9wYW5nby8ubG9jYWwvc2hhcmUvbWN2bS9pbnRlcm5hbC9saWJyYXJpZXMvb3JnL293Mi9hc20vYXNtLWFuYWx5c2lzLzkuOC9hc20tYW5hbHlzaXMtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9vcmcvb3cyL2FzbS9hc20tY29tbW9ucy85LjgvYXNtLWNvbW1vbnMtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9vcmcvb3cyL2FzbS9hc20tdHJlZS85LjgvYXNtLXRyZWUtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9vcmcvb3cyL2FzbS9hc20tdXRpbC85LjgvYXNtLXV0aWwtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9uZXQvZmFicmljbWMvc3BvbmdlLW1peGluLzAuMTUuNSttaXhpbi4wLjguNy9zcG9uZ2UtbWl4aW4tMC4xNS41K21peGluLjAuOC43LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9uZXQvZmFicmljbWMvZmFicmljLWxvYWRlci8wLjE2LjE0L2ZhYnJpYy1sb2FkZXItMC4xNi4xNC5qYXIiLCIvaG9tZS9wYW5nby8ubG9jYWwvc2hhcmUvbWN2bS9pbnRlcm5hbC9saWJyYXJpZXMvbmV0L2ZhYnJpY21jL2ludGVybWVkaWFyeS8xLjIwLjEvaW50ZXJtZWRpYXJ5LTEuMjAuMS5qYXIiXSwiamFyX3BhdGhfb3ZlcnJpZGUiOm51bGwsImp2bV9hcmdzIjpbIi1Ec29kaXVtLmNoZWNrcy5pc3N1ZTI1NjE9ZmFsc2UiXSwibG9hZGVyX3ZlcnNpb24iOm51bGwsIm1haW5fY2xhc3Nfb3ZlcnJpZGUiOiJuZXQuZmFicmljbWMubG9hZGVyLmltcGwubGF1bmNoLmtub3QuS25vdENsaWVudCJ9fQ==\n"],
			&["%_eyJtZXNzYWdlIjp7ImNvbnRlbnRzIjp7IlN1Y2Nlc3MiOiJGYWJyaWMgZG93bmxvYWRlZCJ9LCJsZXZlbCI6ImltcG9ydGFudCJ9fQ==", "%_ImVuZF9wcm9jZXNzIg==", "%_eyJzZXRfcmVzdWx0Ijp7ImNsYXNzcGF0aF9leHRlbnNpb24iOlsiL2hvbWUvcGFuZ28vLmxvY2FsL3NoYXJlL21jdm0vaW50ZXJuYWwvbGlicmFyaWVzL29yZy9vdzIvYXNtL2FzbS85LjgvYXNtLTkuOC5qYXIiLCIvaG9tZS9wYW5nby8ubG9jYWwvc2hhcmUvbWN2bS9pbnRlcm5hbC9saWJyYXJpZXMvb3JnL293Mi9hc20vYXNtLWFuYWx5c2lzLzkuOC9hc20tYW5hbHlzaXMtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9vcmcvb3cyL2FzbS9hc20tY29tbW9ucy85LjgvYXNtLWNvbW1vbnMtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9vcmcvb3cyL2FzbS9hc20tdHJlZS85LjgvYXNtLXRyZWUtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9vcmcvb3cyL2FzbS9hc20tdXRpbC85LjgvYXNtLXV0aWwtOS44LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9uZXQvZmFicmljbWMvc3BvbmdlLW1peGluLzAuMTUuNSttaXhpbi4wLjguNy9zcG9uZ2UtbWl4aW4tMC4xNS41K21peGluLjAuOC43LmphciIsIi9ob21lL3BhbmdvLy5sb2NhbC9zaGFyZS9tY3ZtL2ludGVybmFsL2xpYnJhcmllcy9uZXQvZmFicmljbWMvZmFicmljLWxvYWRlci8wLjE2LjE0L2ZhYnJpYy1sb2FkZXItMC4xNi4xNC5qYXIiLCIvaG9tZS9wYW5nby8ubG9jYWwvc2hhcmUvbWN2bS9pbnRlcm5hbC9saWJyYXJpZXMvbmV0L2ZhYnJpY21jL2ludGVybWVkaWFyeS8xLjIwLjEvaW50ZXJtZWRpYXJ5LTEuMjAuMS5qYXIiXSwiamFyX3BhdGhfb3ZlcnJpZGUiOm51bGwsImp2bV9hcmdzIjpbIi1Ec29kaXVtLmNoZWNrcy5pc3N1ZTI1NjE9ZmFsc2UiXSwibG9hZGVyX3ZlcnNpb24iOm51bGwsIm1haW5fY2xhc3Nfb3ZlcnJpZGUiOiJuZXQuZmFicmljbWMubG9hZGVyLmltcGwubGF1bmNoLmtub3QuS25vdENsaWVudCJ9fQ=="],
		)
		.await;
	}

	async fn test(outputs: &[&'static str], expected_lines: &[&'static str]) {
		let mut reader = TryLineReader::new(TestReader {
			outputs: outputs.into_iter().map(|x| *x).collect(),
		});

		let mut lines = Vec::new();
		while let Some(new_lines) = reader.lines().await.unwrap() {
			lines.extend(new_lines.into_iter().map(|x| x.to_string()));
		}

		let expected_lines: Vec<_> = expected_lines.into_iter().map(|x| x.to_string()).collect();
		assert_eq!(lines, expected_lines);
	}
}
