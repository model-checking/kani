// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks -Z mem-predicates
//! Check that Kani can check value validity of packed structs.

use std::ptr::addr_of;

#[repr(C, packed)]
#[derive(kani::Arbitrary)]
struct Packed {
    byte: u8,
    c: char,
}

#[kani::proof]
pub fn check_packed_deref() {
    let packed: Packed = kani::any();
    assert!(kani::mem::can_dereference(addr_of!(packed)));
    assert!(kani::mem::can_dereference(addr_of!(packed.byte)));
    assert!(!kani::mem::can_dereference(addr_of!(packed.c)));
}

#[kani::proof]
pub fn check_packed_read_unaligned() {
    let packed: Packed = kani::any();
    assert!(kani::mem::can_read_unaligned(addr_of!(packed)));
    assert!(kani::mem::can_read_unaligned(addr_of!(packed.byte)));
    assert!(kani::mem::can_read_unaligned(addr_of!(packed.c)));
}

#[kani::proof]
pub fn check_packed_read_unaligned_invalid_value() {
    const SZ: usize = size_of::<Packed>();
    let val = [u8::MAX; SZ];
    let ptr = addr_of!(val) as *const Packed;
    assert!(!kani::mem::can_read_unaligned(ptr));
}
