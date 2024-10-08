// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(4)]
fn check_abs() {
    let data: [u8; 3] = kani::any();
    let mut string = String::from_utf8_lossy(&data).to_string();
    let new_len = kani::any();
    kani::assume(new_len <= 2);
    string.truncate(new_len);
    assert!(string.len() <= 2);
}
