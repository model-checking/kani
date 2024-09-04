// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that a high offset causes a "wrapping around" behavior in CBMC.

// This example can be really confusing. This program works fine in Rust and
// it's okay to assert that the value coming from `offset_from` is equal to
// `high_offset`. But CBMC's memory model is going to cause a "wrapping around"
// behavior in `v_wrap`, so any values that depend on it are going to show a
// strange behavior as well.
use std::convert::TryInto;

#[kani::proof]
fn main() {
    let v: &[u128] = &[0; 10];
    let v_0: *const u128 = &v[0];
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
        let wrapped_offset = v_wrap.offset_from(v_0);
        // Both offsets should be the same, but because of the "wrapping around"
        // behavior in CBMC, `wrapped_offset` does not equal `high_offset`
        // https://github.com/model-checking/kani/issues/1150
        assert!(high_offset == wrapped_offset.try_into().unwrap());
    }
}
