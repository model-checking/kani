# Changelog

This file contains notable changes (e.g. breaking changes, major changes, etc.) in Kani releases.

This file was introduced starting Kani 0.23.0, so it only contains changes from version 0.23.0 onwards.

## [0.37.0]

### Major Changes

* Delete obsolete stubs for `Vec` and related options by @zhassan-aws in https://github.com/model-checking/kani/pull/2770
* Add support for the ARM64 Linux platform by @adpaco-aws in https://github.com/model-checking/kani/pull/2757

## What's Changed

* Function Contracts: Support for defining and checking `requires` and `ensures` clauses by @JustusAdam in https://github.com/model-checking/kani/pull/2655
* Force `any_vec` capacity to match length by @celinval in https://github.com/model-checking/kani/pull/2765
* Fix expected value for `pref_align_of` under aarch64/macos by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/2782
* Bump CBMC version to 5.92.0 by @zhassan-aws in https://github.com/model-checking/kani/pull/2771
* Upgrade to Kissat 3.1.1 by @zhassan-aws in https://github.com/model-checking/kani/pull/2756
* Update Rust toolchain to nightly-2023-09-19 by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/2778

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.36.0...kani-0.37.0
## [0.36.0]

## What's Changed

* Enable `-Z stubbing` and error out instead of ignoring stub by @celinval in https://github.com/model-checking/kani/pull/2678
* Enable concrete playback for failure of UB checks by @zhassan-aws in https://github.com/model-checking/kani/pull/2727
* Bump CBMC version to 5.91.0 by @adpaco-aws in https://github.com/model-checking/kani/pull/2733
* Rust toolchain upgraded to `nightly-2023-09-06` by @celinval @jaisnan @adpaco-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.35.0...kani-0.36.0

## [0.35.0]

## What's Changed

* Add support to `simd_bitmask` by @celinval in https://github.com/model-checking/kani/pull/2677
* Add integer overflow checking for `simd_div` and `simd_rem` by @reisnera in https://github.com/model-checking/kani/pull/2645
* Bump CBMC version by @zhassan-aws in https://github.com/model-checking/kani/pull/2702
* Upgrade Rust toolchain to 2023-08-19 by @zhassan-aws in https://github.com/model-checking/kani/pull/2696

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.34.0...kani-0.35.0

## [0.34.0]

### Breaking Changes
* Change default solver to CaDiCaL by @celinval in https://github.com/model-checking/kani/pull/2557
By default, Kani will now run CBMC with CaDiCaL, since this solver has outperformed Minisat in most of our benchmarks.
User's should still be able to select Minisat (or a different solver) either by using `#[solver]` harness attribute,
or by passing `--solver=<SOLVER>` command line option.

## What's Changed

* Allow specifying the scheduling strategy in #[kani_proof] for async functions by @fzaiser in https://github.com/model-checking/kani/pull/1661
* Support for stubbing out foreign functions by @feliperodri in https://github.com/model-checking/kani/pull/2658
* Coverage reporting without a need for cbmc-viewer by @adpaco-aws in https://github.com/model-checking/kani/pull/2609
* Add support to array-based SIMD by @celinval in https://github.com/model-checking/kani/pull/2633
* Add unchecked/SIMD bitshift checks and disable CBMC flag by @reisnera in https://github.com/model-checking/kani/pull/2630
* Fix codegen of constant byte slices to address spurious verification failures by @zhassan in https://github.com/model-checking/kani/pull/2663
* Bump CBMC to v5.89.0 by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/2662
* Update Rust toolchain to nightly 2023-08-04 by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/2661

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.33.0...kani-0.34.0

## [0.33.0]

## What's Changed
* Add support for sysconf by @feliperodri in https://github.com/model-checking/kani/pull/2557
* Print Kani version by @adpaco-aws in https://github.com/model-checking/kani/pull/2619
* Upgrade Rust toolchain to nightly-2023-07-01 by @qinheping in https://github.com/model-checking/kani/pull/2616
* Bump CBMC version to 5.88.1 by @zhassan-aws in https://github.com/model-checking/kani/pull/2623

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.32.0...kani-0.33.0

## [0.32.0]

## What's Changed

* Add kani::spawn and an executor to the Kani library by @fzaiser in https://github.com/model-checking/kani/pull/1659
* Add "kani" configuration key to enable conditional compilation in build scripts by @celinval in https://github.com/model-checking/kani/pull/2297
* Adds posix_memalign to the list of supported builtins by @feliperodri in https://github.com/model-checking/kani/pull/2601
* Upgrade rust toolchain to nightly-2023-06-20 by @celinval in https://github.com/model-checking/kani/pull/2551
* Update rust toolchain to 2023-06-22 by @celinval in https://github.com/model-checking/kani/pull/2588
* Automatic toolchain upgrade to nightly-2023-06-24 by @github-actions in https://github.com/model-checking/kani/pull/2600
* Bump CBMC version to 5.87.0 by @adpaco-aws in https://github.com/model-checking/kani/pull/2598

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.31.0...kani-0.32.0

## [0.31.0]

## What's Changed
* Add `--exact` flag by @jaisnan in https://github.com/model-checking/kani/pull/2527
* Build the verification libraries using Kani compiler by @celinval in https://github.com/model-checking/kani/pull/2534
* Verify all Kani attributes in all crate items upfront by @celinval in https://github.com/model-checking/kani/pull/2536
* Update README.md - fix link locations for badge images in markdown by @phayes in https://github.com/model-checking/kani/pull/2537
* Bump CBMC version to 5.86.0 by @zhassan-aws in https://github.com/model-checking/kani/pull/2561

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.30.0...kani-0.31.0

## [0.30.0]

## What's Changed
* Remove --harness requirement from stubbing by @celinval in https://github.com/model-checking/kani/pull/2495
* Add target selection for cargo kani by @celinval in https://github.com/model-checking/kani/pull/2507
* Generate Multiple playback harnesses when multiple crashes exist in a single harness. by @YoshikiTakashima in https://github.com/model-checking/kani/pull/2496
* Escape Zero-size types in playback by @YoshikiTakashima in https://github.com/model-checking/kani/pull/2508
* Do not crash when `rustfmt` fails. by @YoshikiTakashima in https://github.com/model-checking/kani/pull/2511
* De-duplicate same input injections for the same harness. by @YoshikiTakashima in https://github.com/model-checking/kani/pull/2513
* Update Cbmc version  by @celinval in https://github.com/model-checking/kani/pull/2512
* Upgrade rust toolchain to 2023-04-30 by @zhassan-aws in https://github.com/model-checking/kani/pull/2456

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.29.0...kani-0.30.0


## [0.29.0]

### Major Changes
* Create a playback command to make it easier to run Kani generated tests ([pull request](https://github.com/model-checking/kani/pull/2464) by @celinval)

### What Else has Changed
* Fix symtab json file removal and reduce regression scope ([pull request](https://github.com/model-checking/kani/pull/2447) by @celinval)
* Fix regression on concrete playback inplace ([pull request](https://github.com/model-checking/kani/pull/2454) by @celinval)
* Fix static variable initialization when they have the same value ([pull request](https://github.com/model-checking/kani/pull/2469) by @celinval)
* Improve assess and regression time ([pull request](https://github.com/model-checking/kani/pull/2478) by @celinval)
* Fix playback with build scripts ([pull request](https://github.com/model-checking/kani/pull/2477) by @celinval)
* Delay printing playback harness until after verification status ([pull request](https://github.com/model-checking/kani/pull/2480) by @YoshikiTakashima)
* Update rust toolchain to 2023-04-29 ([pull request](https://github.com/model-checking/kani/pull/2452) by @zhassan-aws)
* Bump CBMC version to 5.84.0 ([pull request](https://github.com/model-checking/kani/pull/2483) by @tautschn)

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.28.0...kani-0.29.0

## [0.28.0]

### Breaking Changes
* The unstable `--c-lib` option now requires `-Z c-ffi` to enable C-FFI support by @celinval in https://github.com/model-checking/kani/pull/2425

### What's Changed
* Enforce unstable APIs can only be used if the related feature is enabled by @celinval in https://github.com/model-checking/kani/pull/2386
* Get rid of the legacy mode by @celinval in https://github.com/model-checking/kani/pull/2427
* Limit FFI calls by default to explicitly supported ones by @celinval in https://github.com/model-checking/kani/pull/2428
* Fix the order of operands for generator structs by @zhassan-aws in https://github.com/model-checking/kani/pull/2436
* Add a few options to dump the reachability graph (debug only) by @celinval in https://github.com/model-checking/kani/pull/2433
* Perform reachability analysis on a per-harness basis by @celinval in https://github.com/model-checking/kani/pull/2439
* Bump CBMC version to 5.83.0 by @zhassan-aws in https://github.com/model-checking/kani/pull/2441
* Upgrade the toolchain to nightly-2023-04-16  by @celinval in https://github.com/model-checking/kani/pull/2406

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.27.0...kani-0.28.0

## [0.27.0]

### What's Changed

* Allow excluding packages from verification with `--exclude` by @adpaco-aws in https://github.com/model-checking/kani/pull/2399
* Add size_of annotation to help CBMC's allocator by @tautschnig in https://github.com/model-checking/kani/pull/2395
* Implement `kani::Arbitrary` for `Box<T>` by @adpaco-aws in https://github.com/model-checking/kani/pull/2404
* Use optimized overflow operation everywhere by @celinval in https://github.com/model-checking/kani/pull/2405
* Print compilation stats in verbose mode by @celinval in https://github.com/model-checking/kani/pull/2420
* Bump CBMC version to 5.82.0 by @adpaco-aws in https://github.com/model-checking/kani/pull/2417

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.26.0...kani-0.27.0

## [0.26.0]

### What's Changed

* The Kani reference now includes an ["Attributes"](https://model-checking.github.io/kani/reference/attributes.html) section that describes each of the attributes available in Kani ([pull request](https://github.com/model-checking/kani/pull/2359) by @adpaco-aws)
* Users' choice of SAT solver, specified by the `solver` attribute, is now propagated to the loop-contract synthesizer ([pull request](https://github.com/model-checking/kani/pull/2320) by @qinheping)
* Unit tests generated by the concrete playback feature now compile correctly when using `RUSTFLAGS="--cfg=kani"` ([pull request](https://github.com/model-checking/kani/pull/2353) by @jaisnan)
* The Rust toolchain is updated to 2023-02-18 ([pull request](https://github.com/model-checking/kani/pull/2384) by @tautschnig)

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.25.0...kani-0.26.0


## [0.25.0]

### What's Changed

* Add implementation for the `#[kani::should_panic]` attribute by @adpaco-aws in https://github.com/model-checking/kani/pull/2315
* Upgrade Rust toolchain to nightly-2023-02-04 by @tautschnig in https://github.com/model-checking/kani/pull/2324
* Bump CBMC version to 5.80.0 by @zhassan-aws in https://github.com/model-checking/kani/pull/2336

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.24.0...kani-0.25.0

## [0.23.0]

### Breaking Changes

- Remove the second parameter in the `kani::any_where` function by @zhassan-aws in #2257
We removed the second parameter in the `kani::any_where` function (`_msg: &'static str`) to make the function more ergonomic to use.
We suggest moving the explanation for why the assumption is introduced into a comment.
For example, the following code:
```rust
    let len: usize = kani::any_where(|x| *x < 5, "Restrict the length to a value less than 5");
```
should be replaced by:
```rust
    // Restrict the length to a value less than 5
    let len: usize = kani::any_where(|x| *x < 5);
```

### Major Changes

- Enable the build cache to avoid recompiling crates that haven't changed, and introduce `--force-build` option to compile all crates from scratch by @celinval in #2232.
