// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that autoharness does not deem a struct with a (static) reference field derivable:
// the synthesized Arbitrary implementation would create the referent's storage inside the
// synthesized any() body, and the returned reference would dangle, producing spurious
// "dead object" verification failures.

pub struct HasStaticRef {
    pub r: &'static u32,
}

pub fn read_ref(h: HasStaticRef) -> u32 {
    *h.r
}

// Top-level reference arguments (where the harness owns the storage) remain supported.
pub fn read_direct(r: &u32) -> u32 {
    *r
}
