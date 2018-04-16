## 0.6.0

### Potential Breaking Changes

- There is a small change of breakage if you've relied on `Recursive` using an
  `Arc<BoxedStrategy<T>>` as `Recursive` now internally uses `BoxedStrategy<T>`
  instead as well as expecting a `Fn(BoxedStrategy<T>) -> R` instead of
  `Fn(Arc<BoxedStrategy<T>>) -> R`. In addition, the type of recursive
  strategies has changed from `Recursive<BoxedStrategy<T>, F>` to just
  `Recursive<T, F>`.

### Minor changes

- Reduced indirections and heap allocations inside `Recursive<T, F>` somewhat.

- `BoxedStrategy<T>` and `SBoxedStrategy<T>` now use `Arc` internally instead of
  using `Box`. While this has marginal overhead, it also reduces the overhead
  in `Recursive<T, F>`. The upside to this change is also that you can very
  cheaply clone strategies.

- `Filter` is marginally faster.

### Bug Fixes

- Removed `impl Arbitrary for LocalKeyState` since `LocalKeyState` no longer
  exists in the nightly compiler.

- Unstable features compile on latest nightly again.

## 0.5.1

### New Additions

- `proptest::strategy::Union` and `proptest::strategy::TupleUnion` now work
  with weighted strategies even if the sum of the weights overflows a `u32`.

- Added `SIGNALING_NAN` strategy to generate signalling NaNs if supported by
  the platform. Note that this is _not_ included in `ANY`.

### Bug Fixes

- Fixed values produced via `prop_recursive()` not shrinking from the recursive
  to the non-recursive case.

- Fix that `QUIET_NAN` would generate signalling NaNs on most platforms on Rust
  1.24.0 and later.

## 0.5.0

### Potential Breaking Changes

- There is a small chance of breakage if you've relied on the constraints put
  on type inference by the closure in `leaf.prop_recursive(..)` having a fixed
  output type. The output type is now any strategy that generates the same type
  as `leaf`. This change is intended to make working with recursive types a bit
  easier as you no longer have to use `.boxed()` inside the closure you pass to
  `.prop_recursive(..)`.

- There is a small chance of breakage wrt. type inference due to the
  introduction of `SizeRange`.

- There is a small chance of breakage wrt. type inference due to the
  introduction of `Probability`.

- `BoxedStrategy` and `SBoxedStrategy` are now newtypes instead of being type
  aliases. You will only experience breaking changes if you've directly
  used `.boxed()` and not `(S)BoxedStrategy<T>` but rather
  `Box<Strategy<Value = Box<ValueTree<Value = T>>>>`. The probability of
  breakage is very small, but still possible. The benefit of this change
  is that calling `.boxed()` or `.sboxed()` twice only boxes once. This can
  happen in situations where you have functions `Strategy -> BoxedStrategy` or
  with code generation.

- `proptest::char::ANY` has been removed. Any remaining uses must be replaced
  by `proptest::char::any()`.

- `proptest::strategy::Singleton` has been removed. Any remaining uses must be
  replaced by `proptest::strategy::Just`.

### New Additions

- Proptest now has an `Arbitrary` trait in `proptest::arbitrary` and re-exported
  in the `proptest::prelude`. `Arbitrary` has also been `impl`emented for most
  of the standard library. The trait provides a mechanism to define a canonical
  `Strategy` for a given type just like `Arbitrary` in Haskell's QuickCheck.
  Deriving for this trait will also be provided soon in the crate
  `proptest_derive`. To use the canonical strategy for a certain type `T`,
  you can simply use `any::<T>()`. This is the major new addition of this release.

- The `any_with`, `arbitrary`, `arbitrary_with` free functions in
  the module `proptest::arbitrary`.

- The `ArbitraryF1` and `ArbitraryF2` traits in `proptest::arbitrary::functor`.
  These are "higher order" `Arbitrary` traits that correspond to the `Arbitrary1`
  and `Arbitrary2` type classes in Haskell's QuickCheck. They are mainly provided
  to support a common set of container-like types in custom deriving self-recursive
  types in  `proptest_derive`. More on this later releases.

- The strategies in `proptest::option` and `proptest::result` now accept a type
  `Probability` which is a wrapper around `f64`. Convertions from types such as
  `f64` are provided to make the interface ergonomic to use. Users may also use
  the `proptest::option::prob` function to explicitly construct the type.

- The strategies in `proptest::collections` now accept a type `SizeRange`
  which is a wrapper around `Range<usize>`. Convertions from types
  such as `usize` and `Range<usize>` are provided to make the interface
  ergonomic to use. Users may also use the `proptest::collections::size_bounds`
  function to explicitly construct the type.

- A `.prop_map_into()` operation on all strategies that map
  using `Into<OutputType>`. This is a clerarer and cheaper
  operation than using `.prop_map(OutputType::from)`.

- A nonshrinking `LazyJust` strategy that can be used instead of `Just` when you
  have non-`Clone` types.

- Anything that can be coerced to `fn() -> T` where `T: Debug` is a `Strategy`
  where `ValueFor<fn() -> T> == T`. This is intended to make it easier to reuse
  proptest for unit tests with manual input space partition where `fn() -> T`
  provides fixtures.

### Minor changes

- Relaxed the constraints of `btree_map` removing `'static`.

- Reduced the heap allocation inside `Recursive` somewhat.

## 0.4.2

### Bug Fixes

- The `unstable` feature now works again.

## 0.4.1

### New Additions

- The `proptest::num::f32` and `proptest::num::f64` modules now have additional
  constants (e.g., `POSITIVE`, `SUBNORMAL`, `INFINITE`) which can be used to
  generate subsets of the floating-point domain by class and sign.

### Bug Fixes

- `proptest::num::f32::ANY` and `proptest::num::f64::ANY` now actually produce
  arbitrary values. Previously, they had the same effect as `0.0..1.0`. While
  this fix is a very substantial change in behaviour, it was not considered a
  breaking change since (a) the new behaviour is consistent with the
  documentation and expectations, (b) it's quite unlikely anyone was depending
  on the old behaviour since anyone who wanted that range would have written it
  out, and (c) Proptest isn't generally a transitive dependency so the chance
  of this update happening "by surprise" is low.

## 0.4.0

### Deprecations and Potential Breaking Changes

- `proptest::char::ANY` replaced with `proptest::char::any()`.
  `proptest::char::ANY` is present but deprecated, and will be removed in
  proptest 0.5.0.

- Instead of returning `-> Result<Self::Value, String>`, strategies are
  expected to return `-> Result<Self::Value, Reason>` instead. `Reason` reduces
  the amount of heap allocations, especially for `.prop_filter(..)` where you
  may now also pass in `&'static str`. You will only experience breaks if
  you've written your own strategy types or if you've used
  `TestCaseError::Reject` or `TestCaseError::Fail` explicitly.

- Update of externally-visible crate `rand` to `0.4.2`.

### New Additions

- Added `proptest::test_runner::Reason` which allows you to avoid heap
  allocation in some places and may be used to make the API richer in the
  future without incurring more breaking changes.

- Added a type alias `proptest::strategy::NewTree<S>` where `S: Strategy`
  defined as: `type NewTree<S> = Result<<S as Strategy>::Value, Rejection>`.

## 0.3.4

### Bug Fixes

- Cases where `file!()` returns a relative path, such as on Windows, are now
  handled more reasonably. See
  [#24](https://github.com/AltSysrq/proptest/issues/24) for more details and
  instructions on how to migrate any persistence files that had been written to
  the wrong location.

## 0.3.3

Boxing Day Special

### New Additions

- Added support for `i128` and `u128`. Since this is an unstable feature in
  Rust, this is hidden behind the feature `unstable` which you have to
  explicitly opt into in your `Cargo.toml` file.

- Failing case persistence. By default, when a test fails, Proptest will now
  save the seed for the failing test to a file, and later runs will test the
  persisted failing cases before generating new ones.

- Added `UniformArrayStrategy` and helper functions to simplify generating
  homogeneous arrays with non-`Copy` inner strategies.

- Trait `rand::Rng` and struct `rand::XorShiftRng` are now included in
  `proptest::prelude`.

### Bug Fixes

- Fix a case where certain combinations of strategies, like two
  `prop_shuffle()`s in close proximity, could result in low-quality randomness.

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
