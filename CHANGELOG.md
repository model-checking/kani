## 0.4.0

### Potential Breaking Changes

- Instead of returning `-> Result<Self::Value, String>`, strategies are expected
  to return `-> Result<Self::Value, Rejection>` instead. `Rejection` reduces
  the amount of heap allocations, especially for `.prop_filter(..)` where
  you may now also pass in `&'static str` as well as `Rc<str>`.
  You will only experience breaks if you've written your own strategy types
  or if you've used `TestCaseError::Reject` or `TestCaseError::Fail` expliclty.

### New Additions

- Added `proptest::strategy::Rejection` which allows you to avoid heap allocation
  in some places or share allocation with string-interning.

- Added a type alias `proptest::strategy::NewTree<S>` where `S: Strategy`
  defined as: `type NewTree<S> = Result<<S as Strategy>::Value, Rejection>`.

- Added `TestCaseError::reject` and `reject_case` (short-hand).

- Added `TestCaseError::fail` and `fail_case` (short-hand).

## 0.3.2

### New Additions

- Added `SampledBitSetStrategy` to generate bit sets based on size
  distribution.

- Added `Strategy::sboxed()` and `SBoxedStrategy` to make `Send + Sync` boxed
  strategies.

- `RegexGeneratorStrategy` is now `Send` and `Sync`.

- Added a type alias `ValueFor<S>` where `S: Strategy`. This is a shorter way
  to refer to: `<<S as Strategy>::Value as ValueTree>::Value`.

- Added a type alias `type W<T> = (u32, T)` for a weighted strategy `T` in the
  context of union strategies.

- `TestRunner` now implements `Default`.

- Added `Config::with_cases(number_of_cases: u32) -> Config` for simpler
  construction of a `Config` that only differs by the number of test cases.

- All default fields of `Config` can now be overridden by setting environment
  variables. See the docs of that struct for more details.

- Bumped dependency `rand = "0.3.18"`.

- Added `proptest::sample::subsequence` which returns a strategy generating
  subsequences, of the source `Vec`, with a size within the given `Range`.

- Added `proptest::sample::select` which returns a strategy selecting exactly
  one value from another collection.

- Added `prop_perturb` strategy combinator.

- Added `strategy::check_strategy_sanity()` function to do sanity checks on the
  shrinking implementation of a strategy.

- Added `prop_shuffle` strategy combinator.

- Added `strategy::Fuse` adaptor.

### Bug Fixes

- Fix bug where `Vec`, array and tuple shrinking could corrupt the state of
  their inner values, for example leading to out-of-range integers.

- Fix bug where `Flatten` (a.k.a. the `prop_flat_map` combinator) could fail to
  converge to a failing test case during shrinking.

- Fix `TupleUnion` sometimes panicking during shrinking if there were more than
  two choices.

## 0.3.1

### New Additions

- Added `CharStrategy::new_borrowed`.

## 0.3.0

### New Additions

- `Union` now supports weighting via `Union::new_weighted`. Corresponding
  syntax to specify weights is also available in `prop_oneof!`.

- Added `TupleUnion`, which works like `Union` but permits doing static
  dispatch even with heterogeneous delegate strategies.

- `prop_oneof!` is smarter about how it combines the input strategies.

- Added `option` module to generate weighted or unweighted `Option` types.

- Added `result` module to generate weighted or unweighted `Result` types.

- All `bits` submodules now have a `masked` function to create a strategy for
  generating subsets of an arbitrary bitmask.

### Potential Breaking Changes

- `Union::new` now has a generic argument type which could impact type
  inference.

- The concrete types produced by `prop_oneof!` have changed.

- API functions which used to return `BoxedStrategy` now return a specific
  type.

- `BitSetStrategy<T>` is no longer `Copy` for non-`Copy` types `T` nor `Debug`
  for non-`Debug` types `T`.

- `BitSetLike::max` has been renamed to `BitSetLike::len`.

## 0.2.1

### New Additions

- Added `prop_assert!` macro family to assert without panicking, for quieter
  test failure modes.

- New `prelude` module for easier importing of important things.

- Renamed `Singleton` to `Just`. (The old name is still available.)

- Failure messages produced by `proptest!` are now much more readable.

- Added in-depth tutorial.

## 0.2.0

### Breaking Changes

- `Strategy` now requires `std::fmt::Debug`.

### New Additions

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

## 0.1.1

### New Additions

Add `strategy::NoShrink`, `Strategy::no_shrink()`.
