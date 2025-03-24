// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that Kani can codegen statics that contain pointers to constant (i.e., immutable) allocations.
// Test taken from https://github.com/rust-lang/rust/issues/79738#issuecomment-1946578159

// The MIR is:
// alloc4 (static: BAR, size: 16, align: 8) {
//     ╾─────alloc3<imm>─────╼ 01 00 00 00 00 00 00 00 │ ╾──────╼........
// }

// alloc3 (size: 4, align: 4) {
//     2a 00 00 00                                     │ *...
// }

// alloc1 (static: FOO, size: 16, align: 8) {
//     ╾─────alloc3<imm>─────╼ 01 00 00 00 00 00 00 00 │ ╾──────╼........
// }
pub static FOO: &[i32] = &[42];
pub static BAR: &[i32] = &*FOO;

#[kani::proof]
fn main() {
    assert_eq!(FOO.as_ptr(), BAR.as_ptr());
}
