// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! Test that Kani finds undefined behavior in trait implementation with contracts

const BUFFER_SIZE: usize = 5;
const UNSAFE_OFFSET: isize = 10; // Outside buffer bounds

trait UnsafeOps {
    unsafe fn write_at_offset(&self, ptr: *mut u8, offset: isize, value: u8);
}

struct Handler;

impl UnsafeOps for Handler {
    #[kani::requires(!ptr.is_null())]
    // Missing precondition that offset < BUFFER_SIZE as isize
    #[kani::requires(offset >= 0)]
    unsafe fn write_at_offset(&self, ptr: *mut u8, offset: isize, value: u8) {
        *ptr.offset(offset) = value;
    }
}

#[kani::proof_for_contract(<Handler as UnsafeOps>::write_at_offset)]
fn test_trait_contract_violation() {
    let handler = Handler;
    let mut buffer = [0u8; BUFFER_SIZE];
    let ptr = buffer.as_mut_ptr();

    unsafe {
        handler.write_at_offset(ptr, UNSAFE_OFFSET, 42);
    }
}
