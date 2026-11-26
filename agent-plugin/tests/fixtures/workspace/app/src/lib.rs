// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use ws_core::sum_to;

/// Calls across crates: by path and through a `use`.
pub fn run(n: u32) -> u32 {
    sum_to(n) + ws_core::math::midpoint(n, 3)
}

pub fn run_twice(n: u32) -> u32 {
    run(n) + run(n)
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(5)]
    fn check_run() {
        let n: u32 = kani::any();
        kani::assume(n < 4);
        let _ = run_twice(n);
        kani::cover!(true, "end");
    }
}
