// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright tokio Contributors
// origin: tokio-test/tests/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

#![warn(rust_2018_idioms)]

use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_test::io::Builder;

#[cfg(disabled)] // CBMC takes more than 15 minutes
#[kani::proof]
#[kani::unwind(2)]
async fn read1() {
    let mut mock = Builder::new().read(b"hello ").read(b"world!").build();

    let mut buf = [0; 256];

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"hello ");

    let n = mock.read(&mut buf).await.expect("read 2");
    assert_eq!(&buf[..n], b"world!");
}

#[cfg(disabled)] // CBMC consumes more than 10 GB
#[kani::proof]
#[kani::unwind(2)]
async fn read_error() {
    let error = io::Error::new(io::ErrorKind::Other, "cruel");
    let mut mock = Builder::new().read(b"hello ").read_error(error).read(b"world!").build();
    let mut buf = [0; 256];

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"hello ");

    match mock.read(&mut buf).await {
        Err(error) => {
            assert_eq!(error.kind(), io::ErrorKind::Other);
            assert_eq!("cruel", format!("{}", error));
        }
        Ok(_) => panic!("error not received"),
    }

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"world!");
}

#[cfg(disabled)] // CBMC consumes more than 10 GB
#[kani::proof]
#[kani::unwind(2)]
async fn write1() {
    let mut mock = Builder::new().write(b"hello ").write(b"world!").build();

    mock.write_all(b"hello ").await.expect("write 1");
    mock.write_all(b"world!").await.expect("write 2");
}

#[cfg(disabled)] // CBMC consumes more than 10 GB
#[kani::proof]
#[kani::unwind(2)]
async fn write_error() {
    let error = io::Error::new(io::ErrorKind::Other, "cruel");
    let mut mock = Builder::new().write(b"hello ").write_error(error).write(b"world!").build();
    mock.write_all(b"hello ").await.expect("write 1");

    match mock.write_all(b"whoa").await {
        Err(error) => {
            assert_eq!(error.kind(), io::ErrorKind::Other);
            assert_eq!("cruel", format!("{}", error));
        }
        Ok(_) => panic!("error not received"),
    }

    mock.write_all(b"world!").await.expect("write 2");
}

#[tokio::test]
#[should_panic]
async fn mock_panics_read_data_left() {
    use tokio_test::io::Builder;
    Builder::new().read(b"read").build();
}

#[tokio::test]
#[should_panic]
async fn mock_panics_write_data_left() {
    use tokio_test::io::Builder;
    Builder::new().write(b"write").build();
}
