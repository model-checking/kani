# Proptest

Proptest is a property testing framework (the family of which QuickCheck is
perhaps most well-known) inspired by Hypothesis. It allows to test that certain
properties of your code hold for arbitrary inputs, and if a failure is found,
automatically finds the minimal test case to reproduce the problem. Unlike
QuickCheck, generation and shrinking is defined on a per-value basis instead of
per-type, which makes it much more flexible and simplifies composition.

For a full introduction and examples, see [the
documentation](https://docs.rs/proptest/).

# Status

In my personal usage, everything works pretty well, though the crate itself has
a few rough edges.

There may be breaking changes when "impl Trait" becomes stable or when the
possible restructuring of the `rand` crate occurs.

# Changelog

**0.2.0**: **Breaking changes**: `Strategy` now requires `std::fmt::Debug`. New
  additions:

  - `Strategy` now has a family of `prop_flat_map()` combinators for producing
    dynamic and higher-order strategies.

  - `Strategy` has a `prop_recursive()` combinator which allows generating
    recursive structures.

  - Added `proptest::bool::weighted()` to pull booleans from a weighted
    distribution.

  - New `prop_oneof!` macro makes it easier to select from one of several
    strategies.

  - New `prop_compose!` macro to simplify writing most types of custom
    strategies.

**0.1.1**: Add `strategy::NoShrink`, `Strategy::no_shrink()`.

# Acknowledgements

This crate wouldn't have come into existence had it not been for the [Rust port
of QuickCheck](https://github.com/burntsushi/quickcheck) and the
[`regex_generate`](https://github.com/CryptArchy/regex_generate) crate which
gave wonderful examples of what is possible.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
