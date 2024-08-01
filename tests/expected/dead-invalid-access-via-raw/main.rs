// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test checks an issue reported in github.com/model-checking/kani#3063.
// The access of the raw pointer should fail because the value being dereferenced has gone out of
// scope at the time of access.

#[kani::proof]
pub fn check_invalid_ptr() {
    let raw_ptr = {
        let var = 10;
        &var as *const _
    };

    // This should fail since it is de-referencing a dead object.
    assert_eq!(unsafe { *raw_ptr }, 10);
}
