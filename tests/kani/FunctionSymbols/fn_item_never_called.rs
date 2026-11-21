// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that a function item can be stored in a value without being called. The function item
//! is zero-sized, and reachability does not collect a function that is never called, so codegen
//! must not need the function's symbol. Kani used to panic with "Function ... should've been
//! declared before usage". See <https://github.com/model-checking/kani/issues/5218>.

fn f() -> u8 {
    1
}

struct Holder<F> {
    _f: F,
    x: u8,
}

#[kani::proof]
fn fn_item_in_struct() {
    let h = Holder { _f: f, x: kani::any() };
    assert!(h.x == h.x);
}

#[kani::proof]
fn fn_item_in_tuple() {
    let t = (f, kani::any::<u8>());
    assert!(t.1 == t.1);
}

#[kani::proof]
fn fn_item_captured_by_closure() {
    let g = f;
    let c = move || {
        let _ = &g;
        2u8
    };
    assert_eq!(c(), 2);
}

#[kani::proof]
fn fn_item_in_option() {
    let o = Some(f);
    assert!(o.is_some());
}

#[kani::proof]
fn fn_item_in_array() {
    let a = [f, f];
    assert_eq!(a.len(), 2);
}

#[kani::proof]
fn fn_item_in_struct_called() {
    let h = Holder { _f: f, x: 0 };
    assert_eq!((h._f)() + h.x, 1);
}

#[kani::proof]
fn fn_item_in_option_called() {
    let o = Some(f);
    assert_eq!(o.map(|g| g()), Some(1));
}
