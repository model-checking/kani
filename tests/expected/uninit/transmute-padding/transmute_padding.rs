// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

// 5 bytes of data + 3 bytes of padding.
#[repr(C)]
#[derive(kani::Arbitrary)]
struct S(u32, u8);

/// Checks that Kani catches an attempt to access padding of a struct using transmute.
#[kani::proof]
fn check_uninit_padding() {
    let s = kani::any();
    access_padding(s);
}

fn access_padding(s: S) {
    let _padding: u64 = unsafe { std::mem::transmute(s) }; // ~ERROR: padding bytes are uninitialized, so reading them is UB.
}
