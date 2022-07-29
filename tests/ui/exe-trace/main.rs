// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness proof_harness --enable-unstable --gen-exe-trace

struct MyStruct {
    field1: u8,
    field2: u16,
    field3: u32,
    field4: u64,
}

#[kani::proof]
pub fn proof_harness() {
    let my_struct = MyStruct {
        field1: kani::any(),
        field2: kani::any(),
        field3: kani::any(),
        field4: kani::any(),
    };
    assert!(
        !(my_struct.field1 == 123
            && my_struct.field2 == 124
            && my_struct.field3 == 125
            && my_struct.field4 == 126)
    );
}
