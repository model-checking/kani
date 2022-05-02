// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
struct Fib {
    prev: usize,
    curr: usize,
}

impl Fib {
    fn new() -> Self {
        Fib { prev: 0, curr: 1 }
    }
}

impl Iterator for Fib {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let x = self.prev;
        self.prev = self.curr;
        self.curr += x;
        Some(x)
    }
}

#[kani::proof]
fn main() {
    let mut fib = Fib::new();
    assert!(fib.nth(10).unwrap() == 55);
}
