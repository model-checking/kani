// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand correctly verifies contracts,
// i.e., that it detects the presence of a contract and generates a contract
// harness instead of a standard harness.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

mod should_pass {
    #[kani::requires(divisor != 0)]
    fn div(dividend: u32, divisor: u32) -> u32 {
        dividend / divisor
    }

    #[kani::requires(x != 0 && y != 0)]
    #[kani::ensures(|result : &u8| *result != 0 && x % *result == 0 && y % *result == 0)]
    #[kani::recursion]
    fn has_recursion_gcd(x: u8, y: u8) -> u8 {
        let mut max = x;
        let mut min = y;
        if min > max {
            let val = max;
            max = min;
            min = val;
        }

        let res = max % min;
        if res == 0 { min } else { has_recursion_gcd(min, res) }
    }

    fn has_loop_contract() {
        let mut x: u8 = kani::any_where(|i| *i >= 2);

        #[kani::loop_invariant(x >= 2)]
        while x > 2 {
            x = x - 1;
        }

        assert!(x == 2);
    }

    // Test that we can autoharness functions for unsafe functions with contracts
    #[kani::requires(!left.overflowing_mul(rhs).1)]
    unsafe fn unchecked_mul(left: u8, rhs: u8) -> u8 {
        unsafe { left.unchecked_mul(rhs) }
    }

    // Check that we can create automatic harnesses for more complex situtations, i.e.,
    // functions with contracts that reference nested data structures that derive Arbitrary.
    mod alignment {
        // FIXME: Note that since this is a tuple struct, we generate an extra harness for the Alignment constructor,
        // c.f. https://github.com/model-checking/kani/issues/3832#issuecomment-2730580836
        #[derive(kani::Arbitrary)]
        pub struct Alignment(AlignmentEnum);

        #[derive(kani::Arbitrary)]
        enum AlignmentEnum {
            _Align1Shl0 = 1 << 0,
            _Align1Shl1 = 1 << 1,
            _Align1Shl2 = 1 << 2,
            _Align1Shl3 = 1 << 3,
            _Align1Shl4 = 1 << 4,
            _Align1Shl5 = 1 << 5,
            _Align1Shl6 = 1 << 6,
            _Align1Shl7 = 1 << 7,
            _Align1Shl8 = 1 << 8,
            _Align1Shl9 = 1 << 9,
            _Align1Shl10 = 1 << 10,
            _Align1Shl11 = 1 << 11,
            _Align1Shl12 = 1 << 12,
            _Align1Shl13 = 1 << 13,
            _Align1Shl14 = 1 << 14,
            _Align1Shl15 = 1 << 15,
            _Align1Shl16 = 1 << 16,
            _Align1Shl17 = 1 << 17,
            _Align1Shl18 = 1 << 18,
            _Align1Shl19 = 1 << 19,
            _Align1Shl20 = 1 << 20,
            _Align1Shl21 = 1 << 21,
            _Align1Shl22 = 1 << 22,
            _Align1Shl23 = 1 << 23,
            _Align1Shl24 = 1 << 24,
            _Align1Shl25 = 1 << 25,
            _Align1Shl26 = 1 << 26,
            _Align1Shl27 = 1 << 27,
            _Align1Shl28 = 1 << 28,
            _Align1Shl29 = 1 << 29,
            _Align1Shl30 = 1 << 30,
            _Align1Shl31 = 1 << 31,
            _Align1Shl32 = 1 << 32,
            _Align1Shl33 = 1 << 33,
            _Align1Shl34 = 1 << 34,
            _Align1Shl35 = 1 << 35,
            _Align1Shl36 = 1 << 36,
            _Align1Shl37 = 1 << 37,
            _Align1Shl38 = 1 << 38,
            _Align1Shl39 = 1 << 39,
            _Align1Shl40 = 1 << 40,
            _Align1Shl41 = 1 << 41,
            _Align1Shl42 = 1 << 42,
            _Align1Shl43 = 1 << 43,
            _Align1Shl44 = 1 << 44,
            _Align1Shl45 = 1 << 45,
            _Align1Shl46 = 1 << 46,
            _Align1Shl47 = 1 << 47,
            _Align1Shl48 = 1 << 48,
            _Align1Shl49 = 1 << 49,
            _Align1Shl50 = 1 << 50,
            _Align1Shl51 = 1 << 51,
            _Align1Shl52 = 1 << 52,
            _Align1Shl53 = 1 << 53,
            _Align1Shl54 = 1 << 54,
            _Align1Shl55 = 1 << 55,
            _Align1Shl56 = 1 << 56,
            _Align1Shl57 = 1 << 57,
            _Align1Shl58 = 1 << 58,
            _Align1Shl59 = 1 << 59,
            _Align1Shl60 = 1 << 60,
            _Align1Shl61 = 1 << 61,
            _Align1Shl62 = 1 << 62,
            _Align1Shl63 = 1 << 63,
        }

        impl Alignment {
            #[kani::ensures(|result| result.is_power_of_two())]
            pub fn as_usize(self) -> usize {
                self.0 as usize
            }
        }
    }
}

mod should_fail {
    #[kani::ensures(|result : &u32| *result == x)]
    fn max(x: u32, y: u32) -> u32 {
        if x > y { x } else { y }
    }
}
