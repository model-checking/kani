// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
#[must_use]
struct MustUse {
    // some fields
}

impl MustUse {
  fn new() -> MustUse { MustUse {} }
}

// Violates the `unused_must_use` lint.
MustUse::new();
}