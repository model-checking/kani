// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

const BUFFER: *mut u32 = 0x8000 as *mut u32;
const BUFFER2: *mut u32 = 0x1000 as *mut u32;

#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 8)]
fn test_write() {
    let val = 12;
    unsafe {
        //BUFFER+2 is not in the MMIO region. Expect pointer check failures.
        *(BUFFER.offset(2)) = val;
        //BUFFER2 is not in the MMIO region. Expect pointer check failures.
        *BUFFER2 = val;
    }
}
