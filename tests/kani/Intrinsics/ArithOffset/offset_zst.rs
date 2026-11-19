// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z mem-predicates

#[repr(align(16))]
struct AlignedZst;

#[kani::proof]
#[kani::solver(cvc5)]
fn wrapping_offset_preserves_a_zst_pointer() {
    let mut bytes: [u8; 2] = kani::any();
    let base = bytes.as_mut_ptr();
    let zst = base.cast::<AlignedZst>();
    kani::assert(zst.wrapping_offset(0) == zst, "zero offset preserves the pointer");
    let offset: isize = kani::any();
    let shifted = zst.wrapping_offset(offset).cast::<u8>();
    kani::assert(shifted == base, "address preserved");
    kani::assert(kani::mem::same_allocation(base, shifted), "allocation preserved");
    unsafe { shifted.write(17) };
    kani::assert(bytes[0] == 17, "original element receives the write");
}
