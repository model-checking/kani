// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-restrict-vtable -Z restrict-vtable
// Tracking issue for the need for this flag:
// https://github.com/model-checking/kani/issues/802

use std::any::Any;

// Cast one dynamic trait object type to another, which is legal because Send
// is an auto trait with no associated function (so the underlying vtable is
// the same before and after the cast).

// We can also downcast Any to a backing concrete type.

fn downcast_to_concrete(a: &dyn Any) {
    match a.downcast_ref::<i32>() {
        Some(i) => {
            assert!(*i == 7);
        }
        None => {
            assert!(false);
        }
    }
}

fn downcast_to_fewer_traits(s: &(dyn Any + Send)) {
    let c = s as &dyn Any;
    downcast_to_concrete(c);
}

#[kani::proof]
fn main() {
    let i: i32 = 7;
    downcast_to_fewer_traits(&i);
}
