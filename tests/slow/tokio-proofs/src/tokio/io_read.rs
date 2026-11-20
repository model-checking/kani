// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Original copyright tokio contributors.
// origin: tokio/tests/tokio/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio_test::assert_ok;

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::tokio::support::leaked_buffers::LeakedBuffers;

#[kani::proof]
#[kani::unwind(12)]
async fn read() {
    #[derive(Default)]
    struct Rd {
        poll_cnt: usize,
    }

    impl AsyncRead for Rd {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            assert_eq!(0, self.poll_cnt);
            self.poll_cnt += 1;

            buf.put_slice(b"hello world");
            Poll::Ready(Ok(()))
        }
    }

    let mut buf = Box::new([0; 11]);
    let mut rd = Rd::default();

    let n = assert_ok!(rd.read(&mut buf[..]).await);
    assert_eq!(n, 11);
    assert_eq!(buf[..], b"hello world"[..]);
}

struct BadAsyncRead {
    leaked_buffers: LeakedBuffers,
}

impl BadAsyncRead {
    fn new() -> Self {
        Self { leaked_buffers: LeakedBuffers::new() }
    }
}

impl AsyncRead for BadAsyncRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        *buf = ReadBuf::new(unsafe { self.leaked_buffers.create(buf.capacity()) });
        buf.advance(buf.capacity());

        Poll::Ready(Ok(()))
    }
}

#[kani::proof]
#[kani::unwind(2)]
#[kani::should_panic]
async fn read_buf_bad_async_read() {
    let mut buf = Vec::with_capacity(10);
    BadAsyncRead::new().read_buf(&mut buf).await.unwrap();
}
