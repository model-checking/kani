// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can correctly verify the cedar implementation of `SmolStr`
//! An ICE was initially reported for this case in:
//! <https://github.com/model-checking/kani/issues/3312>

#[kani::proof]
#[kani::unwind(13)]
fn check_new() {
    let data: [u8; 12] = kani::any();
    let res = String::from_utf8(data.into());
    kani::assume(res.is_ok());
    let input: String = res.unwrap();
    smol_str::SmolStr::new(&input);
}
