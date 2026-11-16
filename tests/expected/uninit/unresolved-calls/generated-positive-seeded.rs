// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: --edition 2021 -D warnings

#![feature(fn_traits)]

fn identity(value: u8) -> u8 {
    value
}

// The function item is passed through the generic Fn-trait call below.
#[inline(never)]
fn invoke<F: Fn(u8) -> u8>(function: F) -> u8 {
    std::ops::Fn::call(&function, (42,))
}

struct DropCounter<'a>(&'a std::cell::Cell<u8>);
impl Drop for DropCounter<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

// The copy intrinsic initializes the destination and creates an analysis target.
#[inline(never)]
fn seed() {
    let source = 7u8;
    let mut destination = 0u8;
    unsafe { std::ptr::copy_nonoverlapping(&source, &mut destination, 1) };
    assert_eq!(destination, source);
}

#[kani::proof]
fn generated_function_item() {
    seed();
    assert_eq!(invoke(identity), 42);
}

#[kani::proof]
fn drop_closure_glue() {
    seed();
    let count = std::cell::Cell::new(0);
    let value = DropCounter(&count);
    let closure = move || drop(value);
    std::ops::FnOnce::call_once(closure, ());
    assert_eq!(count.get(), 1);
}
