// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can stub trait implementations.

/// Dummy structure
pub struct Bar;

/// Dummy trait
pub trait Foo {
    fn foo() -> bool {
        false
    }
}

impl Foo for Bar {}

impl Foo for u8 {}

impl<T> Foo for [T] {}

impl Foo for [char; 10] {}

impl Foo for (i32, i32) {}

/// Dummy stub
pub fn stub_foo() -> bool {
    true
}

#[kani::proof]
#[kani::stub(<Bar as Foo>::foo, stub_foo)]
#[kani::stub(<(i32, i32) as Foo>::foo, stub_foo)]
#[kani::stub(<[u32] as Foo>::foo, stub_foo)]
fn stub_trait_methods() {}

// https://github.com/model-checking/kani/issues/2524
mod issue_2524 {

    trait Foo {
        fn foo() -> usize;
    }

    struct Bar;

    impl Foo for Bar {
        fn foo() -> usize {
            1
        }
    }

    fn foo_stub() -> usize {
        2
    }

    #[kani::proof]
    #[kani::stub(Bar::foo, foo_stub)]
    fn my_proof() {
        assert_eq!(Bar::foo(), 2)
    }
}
