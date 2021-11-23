// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-memory-safety-checks

// We use `--no-memory-safety-checks` in this test to avoid getting
// a verification failure:
// [main.pointer_dereference.5] line 16 dereference failure: pointer outside object bounds in *lut_ptr: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307
const DEC_DIGITS_LUT: &'static [u8] = b"ab";
fn main() {
    // The next two assertions don't go through to CBMC
    // 'cos they're constant folded away
    assert!(DEC_DIGITS_LUT[0] == b'a');
    assert!(DEC_DIGITS_LUT[1] == b'b');
    let lut_ptr = DEC_DIGITS_LUT.as_ptr();
    // TODO: with `--pointer-check` we get
    // dereference failure: pointer outside object bounds in *lut_ptr
    unsafe {
        assert!(*lut_ptr == b'a');
    }
}
