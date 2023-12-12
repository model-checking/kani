// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing

fn original<T>() {}

trait Dummy {
    const TRUE: bool = true;
}

fn stub<T: Dummy>() {
    // Do nothing.
    assert!(T::TRUE);
}

#[kani::proof]
#[kani::stub(original, stub)]
fn check_mismatch() {
    original::<String>();
}
