// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Original copyright tokio contributors.
// origin: tokio/tests/tokio/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use tokio::io::AsyncBufReadExt;
use tokio_test::assert_ok;

#[cfg(disabled)] // requires memchr
#[kani::proof]
#[kani::unwind(2)]
async fn lines_inherent() {
    let rd: &[u8] = b"hello\r\nworld\n\n";
    let mut st = rd.lines();

    let b = assert_ok!(st.next_line().await).unwrap();
    assert_eq!(b, "hello");
    let b = assert_ok!(st.next_line().await).unwrap();
    assert_eq!(b, "world");
    let b = assert_ok!(st.next_line().await).unwrap();
    assert_eq!(b, "");
    assert!(assert_ok!(st.next_line().await).is_none());
}
