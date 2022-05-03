// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --unwinding-assertions

use std::io::{self, Read, Write};

type Result<T> = std::result::Result<T, io::Error>;

pub struct MemoryMapping {
    addr: *mut u8,
    size: usize,
}

impl MemoryMapping {
    pub fn new(size: usize) -> Result<MemoryMapping> {
        if kani::any() {
            let mm = MemoryMapping { addr: std::ptr::null_mut(), size: size };
            Ok(mm)
        } else {
            Err(io::Error::from_raw_os_error(1))
        }
    }
}

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let mm = MemoryMapping::new(2);
    if mm.is_ok() {
        let mm = mm.expect("foo");
        assert!(mm.size == 2); //should pass
        assert!(mm.size == 3); //should fail
    }
}
