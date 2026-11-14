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

#[kani::proof]
fn check() {
    let mut value = 0u8;
    indirect(&ByteWriter, &raw mut value);
    assert_eq!(value, 7);
}
