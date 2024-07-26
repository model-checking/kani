// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! This test checks that contracts correctly handles mutable static.

static mut WRAP_COUNTER: Option<u32> = None;

/// This function is safe and should never crash. Counter starts at 0.
#[kani::modifies(std::ptr::addr_of!(WRAP_COUNTER))]
#[kani::ensures(|_| true)]
pub fn next() -> u32 {
    // Safe in single-threaded.
    unsafe {
        match &WRAP_COUNTER {
            Some(val) => {
                WRAP_COUNTER = Some(val.wrapping_add(1));
                *val
            }
            None => {
                WRAP_COUNTER = Some(0);
                0
            }
        }
    }
}

/// This harness should succeed.
///
/// Today, CBMC havocs WRAP_COUNTER, which includes invalid discriminants triggering UB.
#[kani::proof_for_contract(next)]
fn check_next() {
    let _ret = next();
}

/// Without contracts, we can safely verify `next`.
#[kani::proof]
fn check_next_directly() {
    // First check that initial iteration returns 0 (base case).
    let first = next();
    assert_eq!(first, 0);

    // Havoc WRAP_COUNTER and invoke next.
    unsafe { WRAP_COUNTER = kani::any() };
    let ret = next();
    kani::cover!(ret == 0);
}
