// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --gen-exe-trace

//! This test checks that the correct deterministic values are read and formatted into an executable trace unit test case.
//! Note: it uses a line-by-line coding style to make debugging easier.

struct MyStruct {
    // Unsigned types
    u8_field: u8,
    u16_field: u16,
    u32_field: u32,
    u64_field: u64,
    u128_field: u128,
    usize_field: usize,

    // Signed types
    i8_field: i8,
    i16_field: i16,
    i32_field: i32,
    i64_field: i64,
    i128_field: i128,
    isize_field: isize,

    // Float types
    f32_field: f32,
    f64_field: f64,
}

impl MyStruct {
    fn deterministic_default() -> MyStruct {
        MyStruct {
            u8_field: 0,
            u16_field: 0,
            u32_field: 0,
            u64_field: 0,
            u128_field: 0,
            usize_field: 0,

            i8_field: 0,
            i16_field: 0,
            i32_field: 0,
            i64_field: 0,
            i128_field: 0,
            isize_field: 0,

            f32_field: 0.0,
            f64_field: 0.0,
        }
    }

    fn replace_with_kani_any(&mut self) {
        self.u8_field = kani::any();
        self.u16_field = kani::any();
        self.u32_field = kani::any();
        self.u64_field = kani::any();
        self.u128_field = kani::any();
        self.usize_field = kani::any();

        self.i8_field = kani::any();
        self.i16_field = kani::any();
        self.i32_field = kani::any();
        self.i64_field = kani::any();
        self.i128_field = kani::any();
        self.isize_field = kani::any();

        self.f32_field = kani::any();
        self.f64_field = kani::any();
    }

    fn verif_cond(&self) -> bool {
        let mut result = true;
        result = result && self.u8_field == 101;
        result = result && self.u16_field == 102;
        result = result && self.u32_field == 103;
        result = result && self.u64_field == 104;
        result = result && self.u128_field == 105;
        result = result && self.usize_field == 106;

        result = result && self.i8_field == -107;
        result = result && self.i16_field == 108;
        result = result && self.i32_field == -109;
        result = result && self.i64_field == 110;
        result = result && self.i128_field == -111;
        result = result && self.isize_field == 112;

        result = result && self.f32_field == 0.1;
        result = result && self.f64_field == 0.2;
        result
    }
}

#[kani::proof]
pub fn harness() {
    let mut my_struct = MyStruct::deterministic_default();
    my_struct.replace_with_kani_any();
    // Semantics:
    // my_struct.verif_cond(): iff all fields equal, return true.
    // below assertion: negate the output -> iff all fields equal, return false.
    assert!(!my_struct.verif_cond());
}
