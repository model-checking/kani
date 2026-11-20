// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Unions with a field of an uninhabited type used to crash the memory-initialization
//! instrumentation, which assumed every primitive layout is a scalar
//! (<https://github.com/model-checking/kani/issues/4855>). Check that such unions are
//! instrumented correctly: reads of initialized fields verify, while reads of
//! uninitialized bytes are still reported.

#[derive(Copy, Clone)]
enum Never {}

union Foo {
    small: u8,
    big: u64,
    never: Never,
}

#[kani::proof]
fn read_init_ok() {
    let f = Foo { big: 7 };
    assert_eq!(unsafe { f.big }, 7);
}

#[kani::proof]
fn read_uninit_bytes_should_fail() {
    let f = Foo { small: 1 };
    let _x = unsafe { f.big };
}
