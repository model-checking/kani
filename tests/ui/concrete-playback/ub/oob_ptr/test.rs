// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

// This test checks that Kani generates a concrete playback test for UB checks
// (e.g. dereferencing a pointer that is outside the object bounds)

#[kani::proof]
fn oob_ptr() {
    let v = vec![1, 2, 3];
    // BUG: predicate should use strict less-than, i.e. `*idx < v.len()`
    let idx: usize = kani::any_where(|idx| *idx <= v.len());
    let _x = unsafe { *v.get_unchecked(idx) };
}
