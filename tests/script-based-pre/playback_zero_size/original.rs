// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This checks that Kani zero-size playback, 2 of them in a row.

#[kani::proof]
fn any_is_ok() {
    let unit: () = kani::any();
    let unit2: () = kani::any();
    kani::cover!(unit == ());
    kani::cover!(unit2 == ());
}
