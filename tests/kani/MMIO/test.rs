// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(kani)]
const BUFFER: *mut u32 = 0xb8000 as *mut u32;

#[kani::proof]
#[kani::mmio_region(0xb8000, 4)]
fn test_write() {
    let val = 12;
    unsafe {
        core::ptr::write_volatile(BUFFER, val);
    }
}

// #[cfg(kani)]
// #[kani::proof]
// #[kani::mmio_region(0xb8000, 4)]
// #[kani::mmio_region(0xbF000, 4)]
// fn test_write2() {
//     let val = 12;
//     unsafe {
//         core::ptr::write_volatile(BUFFER, val);
//         core::ptr::write_volatile(BUFFER2, val);

//     }
// }