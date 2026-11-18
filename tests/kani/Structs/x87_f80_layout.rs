// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `x87_f80` wraps the `x87_f80_raw` lang item, a struct with no fields whose size rustc
//! defines (10 bytes, padded to 16 by the wrapper on x86_64). Kani used to give the raw
//! struct a size of 0, so the wrapper came out 48 bits wide instead of 128, and any
//! transmute to or from it aborted codegen (model-checking/kani#5191).
#![cfg_attr(target_arch = "x86_64", feature(x87_f80, transmute_neo))]

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use std::arch::x86_64::x87_f80;
    use std::mem::{MaybeUninit, size_of, transmute_neo};

    #[kani::proof]
    fn check_x87_f80_layout() {
        assert_eq!(size_of::<x87_f80>(), 16);
        assert_eq!(size_of::<MaybeUninit<x87_f80>>(), 16);
    }

    #[kani::proof]
    fn check_x87_f80_transmute() {
        let bytes: [u8; 16] = kani::any();
        // The same transmute `core` performs between `MaybeUninit<x87_f80>` and `x87_f80`.
        let v: x87_f80 =
            unsafe { transmute_neo::<MaybeUninit<[u8; 16]>, x87_f80>(MaybeUninit::new(bytes)) };
        let back: [u8; 16] = unsafe { transmute_neo::<x87_f80, [u8; 16]>(v) };
        assert_eq!(back, bytes);
        let m: MaybeUninit<x87_f80> = MaybeUninit::new(v);
        let _: x87_f80 = unsafe { transmute_neo::<MaybeUninit<x87_f80>, x87_f80>(m) };
    }
}

#[cfg(not(target_arch = "x86_64"))]
#[kani::proof]
fn check_x87_f80_not_on_this_target() {}
