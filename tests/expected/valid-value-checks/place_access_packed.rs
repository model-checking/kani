// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

#[repr(C, packed)]
struct Packed {
    prefix: u8,
    value: char,
}

#[repr(C, align(4))]
struct Aligned(Packed);

#[kani::proof]
fn packed_field_access() {
    let value: char = kani::any();
    let storage = Aligned(Packed { prefix: 0, value });
    let ptr = &raw const storage.0;
    let field = unsafe { &raw const (*ptr).value };
    // The field is at offset one from an aligned base: misalignment is guaranteed.
    assert_eq!(field as usize % std::mem::align_of::<char>(), 1);
    let copied = unsafe { (*ptr).value };
    assert_eq!(copied, value);
    assert_eq!(unsafe { field.read_unaligned() }, value);
    kani::cover!(true);
}
