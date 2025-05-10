// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the default loop/recursion bounds for automatic harnesses takes effect
// to terminate harnesses that would otherwise run forever.
// (Technically, the harness timeout could take effect before the unwind bound,
// but we raise the timeout to 5mins to make that unlikely.)

fn infinite_loop() {
    loop {}
}

fn gcd_recursion(x: u64, y: u64) -> u64 {
    let mut max = x;
    let mut min = y;
    if min > max {
        let val = max;
        max = min;
        min = val;
    }
    let res = max % min;
    if res == 0 { min } else { gcd_recursion(min, res + 1) }
}
