// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Vec<T> arguments with qualifying element types (integers/floats) are generated
//! *unbounded*: results hold for all lengths, without --bounded-arguments and without the
//! "(bounded)" marker. Loops over the Vec surface insufficient unwinding bounds as
//! unwinding-assertion failures (c.f. `total`). Other element types keep needing
//! BoundedArbitrary support.

// TEST NOTE: should PASS for ALL lengths (loop-free).
pub fn head(v: Vec<u8>) -> Option<u8> {
    v.first().copied()
}

// TEST NOTE: should PASS, and all cover checks must be SATISFIED (lengths beyond any
// bound and full content ranges are generated).
pub fn coverage(v: Vec<i32>) {
    kani::cover!(v.len() > 100_000, "large lengths reachable");
    kani::cover!(!v.is_empty() && v[0] == i32::MIN, "extreme content reachable");
    kani::cover!(v.is_empty(), "empty vec reachable");
}

// TEST NOTE: should FAIL with an unwinding assertion: the Vec is unbounded, so the default
// loop bound cannot cover it — the incompleteness is signaled rather than silent.
pub fn total(v: Vec<u8>) -> u64 {
    v.iter().map(|&b| b as u64).sum()
}

// TEST NOTE: should PASS for ALL lengths: mutable slices are also unbounded (fresh
// exclusive allocations), and writes through them verify.
pub fn set_first(s: &mut [u8]) {
    if !s.is_empty() {
        s[0] = 42;
        assert_eq!(s[0], 42);
    }
}
