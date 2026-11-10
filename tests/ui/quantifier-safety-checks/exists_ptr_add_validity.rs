// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks --solver cadical

#[kani::proof]
fn pointer_witness() {
    let values = [1u8, 2];
    let ptr = values.as_ptr();
    assert!(kani::exists!(|i in (0, 2)| unsafe { *ptr.add(i) == 2 }));
}
