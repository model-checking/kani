// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Original copyright tokio contributors.
// origin: tokio/tests/tokio/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use tokio::io::AsyncReadExt;
use tokio_test::assert_ok;

#[kani::proof]
#[kani::unwind(12)]
async fn read_exact() {
    let mut buf = Box::new([0; 8]);
    let mut rd: &[u8] = b"hello world";

    let n = assert_ok!(rd.read_exact(&mut buf[..]).await);
    assert_eq!(n, 8);
    assert_eq!(buf[..], b"hello wo"[..]);
}
