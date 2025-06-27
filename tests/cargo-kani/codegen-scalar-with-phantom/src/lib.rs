// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that we can codegen structs with scalar and phantom data.
//!
//! Note: Phantom data is represented with ZSTs, which are already covered by
//! the test `codegen-scalar-with-zsts`, but we include this one as well for
//! completeness.

use std::marker::PhantomData;

pub struct Foo<R> {
    x: u8,
    _t: PhantomData<R>,
}

#[kani::proof]
fn check_phantom_data() {
    const C: Foo<usize> = Foo { x: 0, _t: PhantomData };
    assert_eq!(C.x, 0);
}
