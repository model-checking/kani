// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

// Regression for kani#4908.

fn my_from_iter<I: IntoIterator<Item = char>>(_iter: I) -> String {
    String::new()
}

#[kani::proof]
#[kani::stub(<std::string::String as std::iter::FromIterator<char>>::from_iter, my_from_iter)]
fn stub_string_from_iter() {
    let a: [u16; 2] = kani::any();
    let s: String =
        char::decode_utf16(a.iter().cloned()).map(|r| r.unwrap_or('\u{FFFD}')).collect();
    assert!(s.is_empty());
}

#[kani::proof]
#[kani::should_panic]
#[kani::stub(<std::string::String as std::iter::FromIterator<char>>::from_iter, my_from_iter)]
fn stub_string_from_iter_negative() {
    let a: [u16; 2] = kani::any();
    let s: String =
        char::decode_utf16(a.iter().cloned()).map(|r| r.unwrap_or('\u{FFFD}')).collect();
    assert!(!s.is_empty());
}

fn ignore_extend<I: IntoIterator<Item = char>>(_: &mut String, _: I) {}

#[kani::proof]
#[kani::stub(<std::string::String as std::iter::Extend<char>>::extend, ignore_extend)]
fn stub_string_extend() {
    let mut value = String::new();
    value.extend(['a', 'b']);
    assert!(value.is_empty());
}

#[kani::proof]
#[kani::should_panic]
#[kani::stub(<std::string::String as std::iter::Extend<char>>::extend, ignore_extend)]
fn stub_string_extend_negative() {
    let mut value = String::new();
    value.extend(['a', 'b']);
    assert!(!value.is_empty());
}
