// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Make sure that no false positives are generated when points-to analysis overapproximates
//! aliasing.

#[kani::proof]
fn check_delayed_ub_overapprox() {
    unsafe {
        let mut value: u128 = 0;
        let value_ref = &mut value;
        // Perform a call to the helper before mutable pointer cast. This way, a check inserted into
        // the helper will pass.
        helper(value_ref);
        // Cast between two pointers of different padding, which will mark `value` as a possible
        // delayed UB analysis target.
        let ptr = value_ref as *mut _ as *mut (u8, u32, u64);
        *ptr = (4, 4, 4); // Note that since we never read from `value` after overwriting it, no delayed UB occurs.
        // Create another `value` and call helper. Note that since helper could potentially
        // dereference a delayed-UB pointer, an initialization check will be added to the helper.
        // Hence, delayed UB analysis needs to mark the value as properly initialized in shadow
        // memory to avoid the spurious failure.
        let mut value2: u128 = 0;
        helper(&value2);
    }
}

/// A helper that could trigger delayed UB.
fn helper(reference: &u128) -> bool {
    *reference == 42
}
