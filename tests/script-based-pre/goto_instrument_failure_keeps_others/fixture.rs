// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Two proof harnesses. The fake `goto-instrument` in `bin/` fails for the one whose name
//! contains `instrument_oom`, as if goto-instrument had run out of memory on it.

#[kani::proof]
fn passes() {
    assert!(1 + 1 == 2);
}

#[kani::proof]
fn instrument_oom() {
    assert!(2 + 2 == 4);
}
