// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Original copyright tokio contributors.
// origin: tokio/tests/tokio/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{self, AsyncRead, AsyncReadExt, ReadBuf};
use tokio_test::assert_ok;

use crate::tokio::support::leaked_buffers::LeakedBuffers;

#[kani::proof]
#[kani::unwind(12)]
async fn take() {
    let mut buf = [0; 6];
    let rd: &[u8] = b"hello world";

    let mut rd = rd.take(4);
    let n = assert_ok!(rd.read(&mut buf).await);
    assert_eq!(n, 4);
    assert_eq!(&buf, &b"hell\0\0"[..]);
}

#[kani::proof]
#[kani::unwind(12)]
async fn issue_4435() {
    let mut buf = [0; 8];
    let rd: &[u8] = b"hello world";

    let rd = rd.take(4);
    tokio::pin!(rd);

    let mut read_buf = ReadBuf::new(&mut buf);
    read_buf.put_slice(b"AB");

    futures::future::poll_fn(|cx| rd.as_mut().poll_read(cx, &mut read_buf)).await.unwrap();
    assert_eq!(&buf, &b"ABhell\0\0"[..]);
}

struct BadReader {
    leaked_buffers: LeakedBuffers,
}

impl BadReader {
    fn new() -> Self {
        Self { leaked_buffers: LeakedBuffers::new() }
    }
}

impl AsyncRead for BadReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        read_buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut buf = ReadBuf::new(unsafe { self.leaked_buffers.create(10) });
        buf.put_slice(&[123; 10]);
        *read_buf = buf;

        Poll::Ready(Ok(()))
    }
}

#[kani::proof]
#[kani::unwind(2)]
#[kani::should_panic]
async fn bad_reader_fails() {
    let mut buf = Vec::with_capacity(10);

    BadReader::new().take(10).read_buf(&mut buf).await.unwrap();
}
