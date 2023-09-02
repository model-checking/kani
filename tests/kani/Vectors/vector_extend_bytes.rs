// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that we propertly handle `Vec::extend` with a constant byte slice.
//! This used to fail previously (see
//! https://github.com/model-checking/kani/issues/2656).

#[kani::proof]
fn check_extend_const_byte_slice() {
    const MY_CONSTANT: &[u8] = b"Hi";

    let mut my_vec: Vec<u8> = Vec::new();
    my_vec.extend(MY_CONSTANT);
    assert_eq!(my_vec, [72, 105]);
}
