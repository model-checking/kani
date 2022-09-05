// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Original copyright tokio contributors.
// origin: tokio/tests/tokio/

// edition:2021

#![cfg(feature = "full")]
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

// Kani does not support select yet
#[cfg(disabled)]
#[kani::proof]
#[kani::unwind(2)]
#[tokio::test]
async fn empty_read_is_cooperative() {
    tokio::select! {
        biased;

        _ = async {
            loop {
                let mut buf = [0u8; 4096];
                let _ = tokio::io::empty().read(&mut buf).await;
            }
        } => {},
        _ = tokio::task::yield_now() => {}
    }
}

// Kani does not support select yet
#[cfg(disabled)]
#[kani::proof]
#[kani::unwind(2)]
#[tokio::test]
async fn empty_buf_reads_are_cooperative() {
    tokio::select! {
        biased;

        _ = async {
            loop {
                let mut buf = String::new();
                let _ = tokio::io::empty().read_line(&mut buf).await;
            }
        } => {},
        _ = tokio::task::yield_now() => {}
    }
}
