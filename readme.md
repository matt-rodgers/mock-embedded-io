# mock-embedded-io

Mock implementation of [`embedded-io`] and [`embedded-io-async`] traits.

This is intended for testing higher level protocols or applications written on top of these
traits.

The main types of interest are:
- `Source` : mock object implementing both blocking and async `Read` traits.
- `Sink` : mock object implementing both blocking and async `Write` traits.

These types can be constructed using the builder-style methods to return a desired sequence of
return values and data. In the case of the `Sink`, the data written to it is stored for later
inspection.

## Example

```
use embedded_io::{Read, Write};

let data_bytes = "hello world!".as_bytes();
let mut buf: [u8; 64] = [0; 64];

let mut mock_source = Source::new()
                          .data(data_bytes)
                          .error(MockError(embedded_io::ErrorKind::BrokenPipe));

let res = mock_source.read(&mut buf);
assert!(res.is_ok_and(|n| &buf[0..n] == data_bytes));

let res = mock_source.read(&mut buf);
assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));

let mut mock_sink = Sink::new()
                        .accept_data(12)
                        .error(MockError(embedded_io::ErrorKind::BrokenPipe));

let res = mock_sink.write(data_bytes);
assert!(res.is_ok_and(|n| n == data_bytes.len()));

let res = mock_sink.write(data_bytes);
assert!(res.is_err_and(|e| e == MockError(embedded_io::ErrorKind::BrokenPipe)));

let written = mock_sink.into_inner_data();
assert_eq!(written, data_bytes);
```

[`embedded-io`]: https://docs.rs/embedded-io/latest/embedded_io/
[`embedded-io-async`]: https://docs.rs/embedded-io-async/latest/embedded_io_async/
