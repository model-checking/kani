// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Indexing a slice reaches a call through a function pointer in `core`'s panic formatting. The
//! delayed UB instrumentation, which looks for calls to the copy intrinsics, used to crash on it.

#[kani::proof]
fn check_subslice() {
    let arr: [u32; 4] = [kani::any(), kani::any(), kani::any(), kani::any()];
    let slice: &[u32] = &arr;
    let sub = &slice[1..3];
    assert!(sub[0] < 200 || sub[0] >= 200);
}
