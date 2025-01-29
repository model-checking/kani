// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can correctly verify the cedar implementation of `SmolStr`
//! An ICE was initially reported for this case in:
//! <https://github.com/model-checking/kani/issues/3312>

#[kani::proof]
#[kani::unwind(4)]
fn check_new() {
    let data: [char; 3] = kani::any();
    let input: String = data.iter().collect();
    smol_str::SmolStr::new(&input);
}
