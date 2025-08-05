// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zstubbing
//! Test that Kani finds undefined behavior when stubbing trait function with unsafe function

const BUFFER_SIZE: usize = 5;
const UNSAFE_OFFSET: isize = 10; // Outside buffer bounds

trait SafeOps {
    fn safe_write(&self, ptr: *mut u8, offset: isize, value: u8);
}

struct Handler;

impl SafeOps for Handler {
    fn safe_write(&self, ptr: *mut u8, offset: isize, value: u8) {
        // Safe implementation with bounds check
        if offset >= 0 && offset < BUFFER_SIZE as isize {
            unsafe {
                *ptr.offset(offset) = value;
            }
        }
    }
}

// Unsafe stub function without bounds checking
fn unsafe_stub(_handler: &Handler, ptr: *mut u8, offset: isize, value: u8) {
    unsafe {
        *ptr.offset(offset) = value;
    }
}

#[kani::proof]
#[kani::stub(<Handler as SafeOps>::safe_write, unsafe_stub)]
fn test_unsafe_stub() {
    let handler = Handler;
    let mut buffer = [0u8; BUFFER_SIZE];
    let ptr = buffer.as_mut_ptr();

    // This should be safe with original implementation but causes UB when stubbed
    handler.safe_write(ptr, UNSAFE_OFFSET, 42);
}
