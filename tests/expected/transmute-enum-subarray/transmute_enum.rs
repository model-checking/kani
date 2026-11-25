// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check transmutes of an integer into an enum whose tag is narrower than the integer.
//! rustc's `InvalidEnumConstruction` check reads the tag through a sub-array projection
//! (`[MaybeUninit<u8>; 2]` to `[MaybeUninit<u8>; 1]`), which Kani used to report as unsupported.
//! See <https://github.com/model-checking/kani/issues/5217>.

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(u8)]
enum E {
    A(u8) = 1,
    B = 5,
}

#[kani::proof]
fn transmute_valid_value() {
    let x: u16 = kani::any();
    let [tag, payload] = x.to_le_bytes();
    kani::assume(tag == 1 || tag == 5);
    let e: E = unsafe { std::mem::transmute(x) };
    match e {
        E::A(v) => assert!(tag == 1 && v == payload),
        E::B => assert!(tag == 5),
    }
}

#[kani::proof]
fn transmute_invalid_value_should_fail() {
    let x: u16 = kani::any();
    let [tag, _] = x.to_le_bytes();
    kani::assume(tag == 2);
    let _e: E = unsafe { std::mem::transmute(x) };
}
