// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[repr(C)]
struct Padded {
    byte: u8,
    word: u32,
}

#[inline(never)]
fn read_value(value: &u64) -> u64 {
    *value
}

#[inline(never)]
fn read_indirect(value: &u64) -> u64 {
    read_value(value)
}

#[kani::proof]
fn delayed_harness() {
    let mut value = 0u64;
    unsafe {
        let padded = (&mut value as *mut u64).cast::<Padded>();
        *padded = Padded { byte: 1, word: 2 };
    }
    #[cfg(feature = "delayed-first")]
    let _value = read_indirect(&value);
    #[cfg(not(feature = "delayed-first"))]
    let _value = read_value(&value);
}

#[kani::proof]
fn initialized_control_harness() {
    let value = 7u64;
    #[cfg(feature = "delayed-first")]
    let result = read_value(&value);
    #[cfg(not(feature = "delayed-first"))]
    let result = read_indirect(&value);
    assert_eq!(result, 7);
}
