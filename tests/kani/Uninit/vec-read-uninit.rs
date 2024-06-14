// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#[kani::proof]
#[kani::should_panic]
fn main() {
    let v: Vec<u8> = Vec::with_capacity(10);
    let undef = unsafe { *v.as_ptr().add(5) }; //~ ERROR: uninitialized
    let x = undef + 1;
}
