//! Mock implementation of [`embedded-io`] and [`embedded-io-async`] traits.
//!
//! This is intended for testing higher level protocols or applications written on top of these
//! traits.
//!
//! The main types of interest are:
//! - [`Source`] : mock object implementing both blocking and async `Read` traits.
//! - [`Sink`] : mock object implementing both blocking and async `Write` traits.
//!
//! These types can be constructed using the builder-style methods to return a desired sequence of
//! return values and data. In the case of the `Sink`, the data written to it is stored for later
//! inspection.
//!
//! ## Example
//! ```rust
//! # use mock_embedded_io::{Sink, Source, MockError};
//! use embedded_io::{Read, Write};
//!
//! let data_bytes = "hello world!".as_bytes();
//! let mut buf: [u8; 64] = [0; 64];
//!
//! let mut mock_source = Source::new()
//!                           .data(data_bytes)
//!                           .error(MockError(embedded_io::ErrorKind::BrokenPipe));
//!
//! let res = mock_source.read(&mut buf);
//! assert!(res.is_ok_and(|n| &buf[0..n] == data_bytes));
//!
//! let res = mock_source.read(&mut buf);
//! assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));
//!
//! let mut mock_sink = Sink::new()
//!                         .accept_data(12)
//!                         .error(MockError(embedded_io::ErrorKind::BrokenPipe));
//!
//! let res = mock_sink.write(data_bytes);
//! assert!(res.is_ok_and(|n| n == data_bytes.len()));
//!
//! let res = mock_sink.write(data_bytes);
//! assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));
//!
//! let written = mock_sink.into_inner_data();
//! assert_eq!(written, data_bytes);
//! ```
//!
//! [`embedded-io`]: https://docs.rs/embedded-io/latest/embedded_io/
//! [`embedded-io-async`]: https://docs.rs/embedded-io-async/latest/embedded_io_async/
#![deny(missing_docs)]

use embedded_io::{Error, ErrorKind, ErrorType};
use std::collections::VecDeque;

/// Error type for the crate. This wraps an [`embedded_io::ErrorKind`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MockError(pub ErrorKind);

impl Error for MockError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        self.0
    }
}

/// A value to be yielded by the Source
#[derive(Debug, Clone)]
enum ReadItem {
    /// Yield data to the caller
    Data(Vec<u8>),

    /// Return an error to the caller
    Error(MockError),

    /// Return a data length of zero to the caller
    Closed,
}

/// A value to be yielded by the Sink
#[derive(Debug, Clone)]
enum WriteItem {
    /// Accept data written by the caller up to the given length
    AcceptData(usize),

    /// Return an error to the caller
    Error(MockError),

    /// Close the connection by returning a written length of zero to the caller
    Closed,
}

/// An owned handle to a [`Source`] or [`Sink`].
///
/// It's common to want an object which owns a type implementing `Read` or `Write`. But for testing
/// purposes, the object under test can't consume the mock, as we might want to verify some
/// expectations on it afterwards. To get around this, an `OwnedHandle` can be taken from a [`Source`]
/// or [`Sink`], which also implements the underlying traits but only contains a
/// mutable reference to the underlying mock.
///
/// Once the object under test is dropped, the mock object itself still exists and can be used to
/// verify any expectations.
///
/// ### Example
/// ```rust
/// # use mock_embedded_io::Sink;
/// use embedded_io::Write;
///
/// struct MySerialProtocol<T: embedded_io::Write> {
///     serial: T,
/// }
///
/// impl<T: embedded_io::Write> MySerialProtocol<T> {
///     fn hello(&mut self) -> Result<(), T::Error> {
///         self.serial.write_all("hello".as_bytes())
///     }
/// }
///
/// let mut mock_sink = Sink::new().accept_data(64);
///
/// // MySerialProtocol requires an owned `serial` type here
/// let mut my_protocol = MySerialProtocol {
///     serial: mock_sink.owned_handle(),
/// };
///
/// let res = my_protocol.hello();
/// assert!(res.is_ok());
///
/// // Because we constructed `my_protocol` with an `OwnedHandle`, we still have access to the
/// // original `mock_sink` here to verify that the correct bytes were written.
/// let written = mock_sink.into_inner_data();
/// assert_eq!(written, "hello".as_bytes());
/// ```
#[derive(Debug)]
pub struct OwnedHandle<'a, T> {
    inner: &'a mut T,
}

/// A mock which can act as a data source.
///
/// An instance of the mock can be constructed using the builder-style methods. Each item added by
/// the builder methods will be returned in-order when data is read from the `Source`.
///
/// Items can then be read from it using the [`embedded_io::Read`] or [`embedded_io_async::Read`]
/// traits.
///
/// ### Blocking Example
/// ```rust
/// # use mock_embedded_io::{Source, MockError};
/// use embedded_io::Read;
///
/// let data_bytes = "hello world!".as_bytes();
/// let mut mock_source = Source::new()
///                           .data(data_bytes)
///                           .error(MockError(embedded_io::ErrorKind::BrokenPipe));
///
/// let mut buf: [u8; 64] = [0; 64];
/// let res = mock_source.read(&mut buf);
/// assert!(res.is_ok_and(|n| &buf[0..n] == data_bytes));
///
/// let res = mock_source.read(&mut buf);
/// assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));
/// ```
///
/// ### Async Example
/// ```rust
/// # use mock_embedded_io::{Source, MockError};
/// # #[tokio::main]
/// # async fn main() {
/// use embedded_io_async::Read;
///
/// let data_bytes = "hello world!".as_bytes();
/// let mut mock_source = Source::new()
///                           .data(data_bytes)
///                           .error(MockError(embedded_io::ErrorKind::BrokenPipe));
///
/// let mut buf: [u8; 64] = [0; 64];
/// let res = mock_source.read(&mut buf).await;
/// assert!(res.is_ok_and(|n| &buf[0..n] == data_bytes));
///
/// let res = mock_source.read(&mut buf).await;
/// assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));
/// # }
/// ```
///
/// [`embedded_io::Read`]: https://docs.rs/embedded-io/latest/embedded_io/trait.Read.html
/// [`embedded_io_async::Read`]: https://docs.rs/embedded-io-async/latest/embedded_io_async/trait.Read.html
#[derive(Debug, Default)]
pub struct Source {
    /// A queue of items to return to the caller
    queue: VecDeque<ReadItem>,
}

impl Source {
    /// Create a new empty Source
    pub fn new() -> Self {
        Self::default()
    }

    /// Add data to the source. This can be returned to the caller either in one chunk or
    /// incrementally - for example if 20 bytes of data are added, the caller could read all 20
    /// bytes in one call, or read 10 bytes twice before the `Source` will return the following
    /// item.
    pub fn data<T: Into<Vec<u8>>>(mut self, data: T) -> Self {
        self.queue.push_back(ReadItem::Data(data.into()));
        self
    }

    /// Add an error value to the `Source`.
    pub fn error(mut self, e: MockError) -> Self {
        self.queue.push_back(ReadItem::Error(e));
        self
    }

    /// Add a "connection closed" item to the `Source`. When read, this will return `Ok(0)` to the
    /// caller (which might then result in an error value if they used the [`read_exact`] method
    /// instead of [`read`]).
    ///
    /// [`read`]: https://docs.rs/embedded-io/latest/embedded_io/trait.Read.html#tymethod.read
    /// [`read_exact`]: https://docs.rs/embedded-io/latest/embedded_io/trait.Read.html#method.read_exact
    pub fn closed(mut self) -> Self {
        self.queue.push_back(ReadItem::Closed);
        self
    }

    /// Check if all of the provided items were consumed
    pub fn is_consumed(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get an [`OwnedHandle`] containing the `Source`.
    pub fn owned_handle(&mut self) -> OwnedHandle<Self> {
        OwnedHandle { inner: self }
    }
}

/// A mock which can act as a data sink.
///
/// An instance of the mock can be constructed using the builder-style methods. Each item added by
/// the builder methods will be returned in-order when data is written to the `Sink`.
///
/// Data can then be written to it using the [`embedded_io::Write`] or [`embedded_io_async::Write`]
/// traits.
///
/// ### Blocking Example
/// ```rust
/// # use mock_embedded_io::{Sink, MockError};
/// use embedded_io::Write;
///
/// let mut mock_sink = Sink::new()
///                         .accept_data(12)
///                         .error(MockError(embedded_io::ErrorKind::BrokenPipe));
///
/// let data_bytes = "hello world!".as_bytes();
/// let res = mock_sink.write_all(data_bytes);
/// assert!(res.is_ok());
///
/// let res = mock_sink.write(data_bytes);
/// assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));
/// ```
///
/// ### Async Example
/// ```rust
/// # use mock_embedded_io::{Sink, MockError};
/// # #[tokio::main]
/// # async fn main() {
/// use embedded_io_async::Write;
///
/// let mut mock_sink = Sink::new()
///                         .accept_data(12)
///                         .error(MockError(embedded_io::ErrorKind::BrokenPipe));
///
/// let data_bytes = "hello world!".as_bytes();
/// let res = mock_sink.write_all(data_bytes).await;
/// assert!(res.is_ok());
///
/// let res = mock_sink.write(data_bytes).await;
/// assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));
/// # }
/// ```
///
/// [`embedded_io::Write`]: https://docs.rs/embedded-io/latest/embedded_io/trait.Read.html
/// [`embedded_io_async::Write`]: https://docs.rs/embedded-io-async/latest/embedded_io_async/trait.Read.html
#[derive(Debug, Default)]
pub struct Sink {
    /// A queue of items to return to the caller
    queue: VecDeque<WriteItem>,

    /// The data that has been received from the writer
    data: Vec<u8>,
}

impl Sink {
    /// Create a new empty Sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept n bytes of data written to the Sink
    pub fn accept_data(mut self, n: usize) -> Self {
        self.queue.push_back(WriteItem::AcceptData(n));
        self
    }

    /// Add an error value to the `Sink`
    pub fn error(mut self, e: MockError) -> Self {
        self.queue.push_back(WriteItem::Error(e));
        self
    }

    /// Add a "connection closed" item to the `Sink`. When written, this will return `Ok(0)` to the
    /// caller (which might then result in an error value if they used the [`write_all`] method
    /// instead of [`write`]).
    ///
    /// [`write`]: https://docs.rs/embedded-io/latest/embedded_io/trait.Write.html#tymethod.write
    /// [`write_all`]: https://docs.rs/embedded-io/latest/embedded_io/trait.Write.html#method.write_all
    pub fn closed(mut self) -> Self {
        self.queue.push_back(WriteItem::Closed);
        self
    }

    /// Check if all of the provided items were consumed
    pub fn is_consumed(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get the inner data that has been received from the writer
    pub fn into_inner_data(self) -> Vec<u8> {
        self.data
    }

    /// Get an [`OwnedHandle`] containing the `Sink`
    pub fn owned_handle(&mut self) -> OwnedHandle<Self> {
        OwnedHandle { inner: self }
    }
}

impl ErrorType for Source {
    type Error = MockError;
}

impl ErrorType for Sink {
    type Error = MockError;
}

impl embedded_io::Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let next_item = self
            .queue
            .pop_front()
            .expect("The caller tried to read data, but the Source is completely consumed");

        match next_item {
            ReadItem::Data(data) => {
                let n = buf.len().min(data.len());
                let (to_send, to_pend) = data.split_at(n);

                // If we can't send all the data to the caller, put some back in the queue
                if to_pend.len() > 0 {
                    self.queue.push_front(ReadItem::Data(Vec::from(to_pend)));
                }

                buf[0..n].copy_from_slice(to_send);
                Ok(n)
            }
            ReadItem::Error(e) => Err(e),
            ReadItem::Closed => Ok(0),
        }
    }
}

impl embedded_io_async::Read for Source {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        embedded_io::Read::read(self, buf)
    }
}

impl embedded_io::Write for Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let next_chunk = self
            .queue
            .pop_front()
            .expect("The caller tried to write data, but the Sink is completely consumed");

        match next_chunk {
            WriteItem::AcceptData(maxsize) => {
                let n = buf.len().min(maxsize);
                let remaining = maxsize - n;

                // If the max size wasn't written, push the remaining length back to the queue
                if remaining > 0 {
                    self.queue.push_front(WriteItem::AcceptData(remaining));
                }

                self.data.extend_from_slice(buf);
                Ok(n)
            }
            WriteItem::Error(e) => Err(e),
            WriteItem::Closed => Ok(0),
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl embedded_io_async::Write for Sink {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        embedded_io::Write::write(self, buf)
    }
}

impl<T: ErrorType> ErrorType for OwnedHandle<'_, T> {
    type Error = T::Error;
}

impl<T: embedded_io::Write> embedded_io::Write for OwnedHandle<'_, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

impl<T: embedded_io_async::Write> embedded_io_async::Write for OwnedHandle<'_, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf).await
    }
}

impl<T: embedded_io::Read> embedded_io::Read for OwnedHandle<'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf)
    }
}

impl<T: embedded_io_async::Read> embedded_io_async::Read for OwnedHandle<'_, T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf).await
    }
}
