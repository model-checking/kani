// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
type T = u8;

/// Euclid's algorithm for calculating the GCD of two numbers
#[kani::requires(x != 0 && y != 0)]
#[kani::ensures(result != 0 && x % result == 0 && y % result == 0)]
fn gcd(x: T, y: T) -> T {
    let mut max = x;
    let mut min = y;
    if min > max {
        let val = max;
        max = min;
        min = val;
    }

    loop {
        let res = max % min;
        if res == 0 {
            return min;
        }

        max = min;
        min = res;
    }
}

struct Frac {
    pub num: T,
    pub den: T,
}

impl Frac {
    // constructor
    pub fn new(num: T, den: T) -> Self {
        Frac { num, den }
    }

    /// Method to simplify fraction
    /// For example, `Frac { num: 10, den: 15 }` gets simplified to
    ///     `Frac { num: 2, num: 3 }`
    pub fn simplify(&self) -> Frac {
        let gcd = gcd(self.num, self.den);
        Frac::new(self.num / gcd, self.den / gcd)
    }

    pub fn check_equals(&self, f2: Frac) {
        assert_eq!(self.num % f2.num, 0);
        assert_eq!(self.den % f2.den, 0);
        let gcd1 = self.num / f2.num;
        let gcd2 = self.den / f2.den;
        assert_eq!(gcd1, gcd2);
    }
}

#[kani::proof_for_contract(gcd)]
#[kani::unwind(12)]
fn main() {
    // Needed to avoid having `free` be removed as unused function. This is
    // because DFCC contract enforcement assumes that a definition for `free`
    // exists.
    let _ = Box::new(9_usize);
    let num: T = kani::any();
    let den: T = kani::any();
    kani::assume(num != 0);
    kani::assume(den != 0);
    let frac = Frac::new(num, den);
    let simplified_frac = frac.simplify();
    frac.check_equals(simplified_frac);
}
