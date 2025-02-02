// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoverify subcommand correctly verifies contracts,
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
}

mod should_fail {
    #[kani::ensures(|result : &u32| *result == x)]
    fn max(x: u32, y: u32) -> u32 {
        if x > y { x } else { y }
    }
}
