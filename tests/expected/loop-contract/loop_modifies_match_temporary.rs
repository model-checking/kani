// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! The scrutinee of a `match` with `return` arms is a temporary that the user cannot name.
//! It does not need to be in an explicit `#[kani::loop_modifies]` clause (the shape of the
//! provided `Searcher::next_reject` in `core::str::pattern`).

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

enum Step {
    Hit(usize),
    Miss,
    Done,
}

struct Cursor {
    pos: usize,
    end: usize,
}

impl Cursor {
    fn step(&mut self) -> Step {
        if self.pos >= self.end {
            return Step::Done;
        }
        self.pos += 1;
        if kani::any() { Step::Hit(self.pos) } else { Step::Miss }
    }
}

fn next_hit(c: &mut Cursor) -> Option<usize> {
    #[kani::loop_invariant(c.pos <= c.end)]
    #[kani::loop_modifies(&c.pos)]
    loop {
        match c.step() {
            Step::Hit(pos) => return Some(pos),
            Step::Done => return None,
            _ => continue,
        }
    }
}

#[kani::proof]
fn main() {
    let end: usize = kani::any();
    let mut c = Cursor { pos: 0, end };
    if let Some(pos) = next_hit(&mut c) {
        assert!(0 < pos && pos <= end);
    }
}
