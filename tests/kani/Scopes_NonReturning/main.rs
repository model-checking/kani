// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let a: u32 = kani::any();
    let b = a / 2;
    let c = a / 2;
    {
        let c = c + 1;
        assert!(c > b);
    }
    assert!(c == b);
}
