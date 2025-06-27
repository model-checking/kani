// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

struct MyStruct {
    field1: u8,
    field2: u16,
}

impl kani::Arbitrary for MyStruct {
    fn any() -> Self {
        MyStruct { field1: kani::any(), field2: kani::any() }
    }
}

#[kani::proof]
pub fn harness() {
    let my_struct: MyStruct = kani::any();
    assert!(!(my_struct.field1 == 101 && my_struct.field2 == 102));
}
