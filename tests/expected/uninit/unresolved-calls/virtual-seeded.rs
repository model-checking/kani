// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

trait Writer {
    fn write(&self, pointer: *mut u8);
}
struct ByteWriter;
impl Writer for ByteWriter {
    fn write(&self, pointer: *mut u8) {
        unsafe { *pointer = 7 };
    }
}

#[inline(never)]
fn indirect(writer: &dyn Writer, pointer: *mut u8) {
    writer.write(pointer);
}

// A copy intrinsic creates a delayed-UB analysis target without introducing UB.
#[inline(never)]
fn seed() {
    let source = 7u8;
    let mut destination = 0u8;
    unsafe { std::ptr::copy_nonoverlapping(&source, &mut destination, 1) };
    assert_eq!(destination, source);
}

#[kani::proof]
fn check() {
    seed();
    let mut value = 0u8;
    indirect(&ByteWriter, &raw mut value);
    assert_eq!(value, 7);
}
