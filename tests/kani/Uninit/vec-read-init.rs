// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#[kani::proof]
fn main() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe { *v.as_mut_ptr().add(5) = 0x42 };
    let def = unsafe { *v.as_ptr().add(5) }; // Not UB since accessing initialized memory.
    let x = def + 1;
    // However, uninit memory is read on drop_in_place; not clear whether this should count as UB (MIRI doesn't catch it).
}
