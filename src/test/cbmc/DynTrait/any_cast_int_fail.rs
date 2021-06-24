// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::any::Any;

include!("../../rmc-prelude.rs");

// Cast one dynamic trait object type to another, which is legal because Send
// is an auto trait with no associated function (so the underlying vtable is
// the same before and after the cast).

// We can also downcast Any to a backing concrete type.
// Inverted assert for _fail test.

fn downcast_to_concrete(a: &dyn Any) {
    match a.downcast_ref::<i32>() {
        Some(i) => {
            __VERIFIER_expect_fail(*i == 8, "Wrong underlying concrete value");
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

fn main() {
    let i: i32 = 7;
    downcast_to_fewer_traits(&i);
}
