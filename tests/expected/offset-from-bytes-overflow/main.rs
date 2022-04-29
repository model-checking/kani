// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that an offset computed with `offset_from` triggers a verification failure
// if it overflows an `isize` in bytes.

// This example can be really confusing, but is the only way to reproduce the
// failure we are looking for. This program works fine in Rust and it's even
// possible to assert that the value coming from `offset_from` is equal to
// `high_offset`. But CBMC's memory model is going to cause a "wrapping around"
// behavior in `v_wrap`, so any values that depend on it are going to show a
// strange behavior as well.
use std::convert::TryInto;

#[kani::proof]
fn main() {
    let v: &[u128] = &[0; 10];
    let v_0: *const u128 = &v[0];
    // `high_offset` is an offset that is high enough to:
    //  * Not trigger failures in `v_0.add(...)`
    //  * Trigger failures in `offset_from` after it
    let high_offset = usize::MAX / (std::mem::size_of::<u128>() * 4);
    unsafe {
        // Adding `high offset` to `v_0` is undefined behavior, but Kani's
        // default behavior does not report it. This kind of operations
        // are quite common in the standard library, and we disabled such
        // checks in order to avoid spurious verification failures.
        //
        // Note that this instance of undefined behavior will be reported
        // by `miri` and also by Kani with `--extra-pointer-checks`.
        // Also, dereferencing the pointer will also be reported by Kani's
        // default behavior.
        let v_wrap: *const u128 = v_0.add(high_offset.try_into().unwrap());
        let _ = v_wrap.offset_from(v_0);
    }
}
