// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Property that dropping enum drops exactly 1 case.

static mut CELL: i32 = 0;

struct IncrementCELLWhenDropped {
    increment_by: i32,
}

impl Drop for IncrementCELLWhenDropped {
    fn drop(&mut self) {
        unsafe {
            CELL += self.increment_by;
        }
    }
}

enum EnumWithTwoIncrements {
    Add1(IncrementCELLWhenDropped),
    Add2(IncrementCELLWhenDropped),
}

fn get_random_enum_variant() -> EnumWithTwoIncrements {
    if kani::any() {
        EnumWithTwoIncrements::Add1(IncrementCELLWhenDropped { increment_by: 1 })
    } else {
        EnumWithTwoIncrements::Add2(IncrementCELLWhenDropped { increment_by: 2 })
    }
}

#[kani::proof]
fn main() {
    {
        let _e1 = get_random_enum_variant();
    }
    unsafe {
        assert!(CELL == 1 || CELL == 2);
    }
}
