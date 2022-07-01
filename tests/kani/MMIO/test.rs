// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

const BUFFER: *mut u32 = 0x8000 as *mut u32;
const BUFFER2: *mut u32 = 0x1000 as *mut u32;

#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 4)]
fn test_write() {
    let val = 12;
    unsafe {
        core::ptr::write_volatile(BUFFER, val);
    }
}

#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 4)]
#[kani::mmio_region(0x1000, 4)]
fn test_write2() {
    let val = 12;
    unsafe {
        core::ptr::write_volatile(BUFFER, val);
        core::ptr::write_volatile(BUFFER2, val);

    }
}

#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 8)]
/// Check that larger MMIO regions also work
fn test_write3() {
    let val = 12;
    unsafe {
        *BUFFER = val;
        *(BUFFER.offset(1)) = val;
    }
}