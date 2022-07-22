// Original Copyright tokio contributors. Licensed under the MIT license.
// SPDX-License-Identifier: MIT
// origin: tokio/tests/tokio/
// Modifications Copyright Kani contributors. Licensed under the MIT license.

#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use tokio::io::AsyncBufReadExt;
use tokio_test::assert_ok;

#[tokio::test]
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
