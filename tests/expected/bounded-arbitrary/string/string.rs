// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(6)]
fn check_string() {
    let s: String = kani::bounded_any::<_, 5>();

    kani::cover!(s == String::from(""));
    kani::cover!(s == String::from("a"));
    kani::cover!(s == String::from("ab"));
    kani::cover!(s == String::from("abc"));
    kani::cover!(s == String::from("abcd"));
    kani::cover!(s == String::from("abcde"));
}
