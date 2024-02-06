// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn f(a: u64) -> u64 {
    const C: f64 = 0.618;
    (a as f64).powf(C) as u64
}

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn verify_f() {
        const LIMIT: u64 = 10;
        let x: u64 = kani::any();
        let y: u64 = kani::any();
        // outside these limits our approximation may yield spurious results
        kani::assume(x > LIMIT && x < LIMIT * 3);
        kani::assume(y > LIMIT && y < LIMIT * 3);
        kani::assume(x > y);
        let x_ = f(x);
        let y_ = f(y);
        assert!(x_ >= y_);
    }
}
