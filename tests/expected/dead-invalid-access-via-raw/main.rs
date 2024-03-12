// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
pub fn check_invalid_ptr() {
    let raw_ptr = {
        let var = 10;
        &var as *const _
    };
    // This should fail since it is de-referencing a dead object.
    assert_eq!(unsafe { *raw_ptr }, 10);
}
