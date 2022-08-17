// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//
// NOTE: The initial fix for this has been reverted from rustc. I'm keeping this test here so we
// will know when it has been reverted back.
// kani-check-fail

//! Tests that check handling of opaque casts. Tests were adapted from the rustc repository.

#![feature(type_alias_impl_trait)]

#[derive(Copy, Clone)]
struct Foo((u32, u32));

/// Adapted from:
/// <https://github.com/rust-lang/rust/blob/29c5a028b0c92aa5da6a8eb6d6585a389fcf1035/src/test/ui/type-alias-impl-trait/issue-96572-unconstrained-upvar.rs>
#[kani::proof]
fn check_unconstrained_upvar() {
    type T = impl Copy;
    let foo: T = Foo((1u32, 2u32));
    let x = move || {
        let Foo((a, b)) = foo;
        assert_eq!(a, 1u32);
        assert_eq!(b, 2u32);
    };
}

/// Adapted from:
/// <https://github.com/rust-lang/rust/blob/29c5a028b0c92aa5da6a8eb6d6585a389fcf1035/src/test/ui/type-alias-impl-trait/issue-96572-unconstrained-struct.rs>
#[kani::proof]
fn check_unconstrained_struct() {
    type U = impl Copy;
    let foo: U = Foo((1u32, 2u32));
    let Foo((a, b)) = foo;
    assert_eq!(a, 1u32);
    assert_eq!(b, 2u32);
}

/// Adapted from:
/// <https://github.com/rust-lang/rust/issues/96572#issuecomment-1125117692>
#[kani::proof]
fn check_unpack_option_tuple() {
    type T = impl Copy;
    let foo: T = Some((1u32, 2u32));
    match foo {
        None => (),
        Some((a, b)) => {
            assert_eq!(a, 1u32);
            assert_eq!(b, 2u32)
        }
    }
}
