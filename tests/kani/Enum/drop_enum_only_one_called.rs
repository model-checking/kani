// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check drop implementation for an &dyn dynamic trait object.

static mut CELL: i32 = 0;

trait T {
    fn t(&self) {}
}

enum EnumWithDifferentDrop {
    Add1,
    Add2,
}

impl Drop for EnumWithDifferentDrop {
    fn drop(&mut self) {
        unsafe {
            match self {
                EnumWithDifferentDrop::Add1 => CELL += 1,
                EnumWithDifferentDrop::Add2 => CELL += 2,
            }
        }
    }
}

fn get_random_enum_variant(random: u32) -> EnumWithDifferentDrop {
    if random % 2 == 0 { EnumWithDifferentDrop::Add1 } else { EnumWithDifferentDrop::Add2 }
}

#[kani::proof]
fn main() {
    {
        let _e1 = get_random_enum_variant(kani::any());
    }
    unsafe {
        assert!(CELL == 1 || CELL == 2);
        assert!(CELL != 3);
    }
}
