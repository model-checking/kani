// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

const BUFFER: *mut u32 = 0x8000 as *mut u32;

#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 4)]
fn test_read_after_write_doesnt_havoc() {
    let val = 12;
    unsafe {
        core::ptr::write_volatile(BUFFER, val);
        let new_val = core::ptr::read_volatile(BUFFER);
        assert_eq!(new_val, val);
    }
}

#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 4)]
fn test_read_is_stable() {
    unsafe {
        let val1 = core::ptr::read_volatile(BUFFER);
        let val2 = core::ptr::read_volatile(BUFFER);
        assert_eq!(val1, val2);
    }
}

//TODO Weirdly, these fail even tho the other tests pass???
#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 16)]
fn test_writes_dont_alias() {
    let val1 = 42;
    let val2 = 314;
    unsafe {
        let p1 = BUFFER;
        let p2 = BUFFER.offset(2);
        core::ptr::write_volatile(p1, val1);
        core::ptr::write_volatile(p2, val2);
        assert_eq!(core::ptr::read_volatile(p1), val1);
        assert_eq!(core::ptr::read_volatile(p2), val2);
    }
}

//TODO Weirdly, these fail even tho the other tests pass???
#[cfg(kani)]
#[kani::proof]
#[kani::mmio_region(0x8000, 16)]
fn test_writes_dont_alias2() {
    let val1 = 42;
    let val2 = 314;
    unsafe {
        let p1 = BUFFER;
        let p2 = BUFFER.offset(2);
        *p1 = val1;
        *p2 = val2;
        assert_eq!(*p1, val1);
        assert_eq!(*p2, val2);
        assert_eq!(*p2, val1);
        assert_eq!(*p1, val2);
    }
}
