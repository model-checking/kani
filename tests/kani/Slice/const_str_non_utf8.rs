// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check a constant `&str` over the valid part of a byte array that is not UTF-8 as a whole.
//! Decoding the whole array used to crash the compiler.

const BYTES: &[u8] = &[0xff, b'a', b'b', 0xff];
const STR: &str = unsafe { std::str::from_utf8_unchecked(BYTES.split_at(1).1.split_at(2).0) };

#[kani::proof]
fn check_str_in_bytes() {
    assert_eq!(STR.len(), 2);
    assert_eq!(STR, "ab");
}
