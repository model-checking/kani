# Changelog

This file contains notable changes (e.g. breaking changes, major changes, etc.) in Kani releases.

This file was introduced starting Kani 0.23.0, so it only contains changes from version 0.23.0 onwards.

## [0.53.0]

### Major Changes
* The `--visualize` option is being deprecated and will be removed in a future release. Consider using the `--concrete-playback` option instead.
* The `-Z ptr-to-ref-cast-checks` option is being introduced to check pointer validity when casting raw pointers to references. The feature is currently behind an unstable flag but is expected to be stabilized in the next release once remaining performance issues have been resolved.

### What's Changed

* Add `kani_core` library placeholder and build logic by @celinval in https://github.com/model-checking/kani/pull/3227
* Change ensures into closures by @pi314mm in https://github.com/model-checking/kani/pull/3207
* (Re)introduce `Invariant` trait by @adpaco-aws in https://github.com/model-checking/kani/pull/3190
* Remove empty box creation from contracts impl by @celinval in https://github.com/model-checking/kani/pull/3233
* Add a new verify-std subcommand to Kani by @celinval in https://github.com/model-checking/kani/pull/3231
* Inject pointer validity check when casting raw pointers to references by @artemagvanian in https://github.com/model-checking/kani/pull/3221
* Do not turn trivially diverging loops into assume(false) by @tautschnig in https://github.com/model-checking/kani/pull/3223
* Fix "unused mut" warnings created by generated code. by @jsalzbergedu in https://github.com/model-checking/kani/pull/3247
* Refactor stubbing so Kani compiler only invoke rustc once per crate by @celinval in https://github.com/model-checking/kani/pull/3245
* Use cfg=kani_host for host crates by @tautschnig in https://github.com/model-checking/kani/pull/3244
* Add intrinsics and Arbitrary support for no_core by @jaisnan in https://github.com/model-checking/kani/pull/3230
* Fix a few issues with std verification by @celinval in https://github.com/model-checking/kani/pull/3255
* Fix contract of constant fn with effect feature by @celinval in https://github.com/model-checking/kani/pull/3259
* Fix a few more issues with the std library by @celinval in https://github.com/model-checking/kani/pull/3261
* Fix typed_swap for ZSTs by @tautschnig in https://github.com/model-checking/kani/pull/3256
* Remove further uses of Location::none by @tautschnig in https://github.com/model-checking/kani/pull/3253
* Add a `#[derive(Invariant)]` macro by @adpaco-aws in https://github.com/model-checking/kani/pull/3250
* Contracts: History Expressions via "old" monad by @pi314mm in https://github.com/model-checking/kani/pull/3232
* Function Contracts: remove instances of _renamed by @pi314mm in https://github.com/model-checking/kani/pull/3274
* Remove support for the unstable argument `--function` by @celinval in https://github.com/model-checking/kani/pull/3278
* Deprecate `--visualize` in favor of concrete playback by @celinval in https://github.com/model-checking/kani/pull/3281
* Fix operand in fat pointer comparison by @pi314mm in https://github.com/model-checking/kani/pull/3297
* C library: declare malloc by @tautschnig in https://github.com/model-checking/kani/pull/3296
* Function Contracts: Closure Type Inference by @pi314mm in https://github.com/model-checking/kani/pull/3307
* Add support for f16 and f128 for toolchain upgrade to 6/28 by @jaisnan in https://github.com/model-checking/kani/pull/3306
* Towards Proving Memory Initialization by @artemagvanian in https://github.com/model-checking/kani/pull/3264
* Rust toolchain upgraded to `nightly-2024-07-01` by @tautschnig @celinval @jaisnan @adpaco-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.52.0...kani-0.53.0

## [0.52.0]

## What's Changed
* New section about linter configuraton checking in the doc by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/3198
* Fix `{,e}println!()` by @GrigorenkoPV in https://github.com/model-checking/kani/pull/3209
* Contracts for a few core functions by @celinval in https://github.com/model-checking/kani/pull/3107
* Add simple API for shadow memory by @zhassan-aws in https://github.com/model-checking/kani/pull/3200
* Upgrade Rust toolchain to 2024-05-28 by @zhassan-aws @remi-delmas-3000 @qinheping

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.51.0...kani-0.52.0

## [0.51.0]

### What's Changed

* Do not assume that ZST-typed symbols refer to unique objects by @tautschnig in https://github.com/model-checking/kani/pull/3134
* Remove `kani::Arbitrary` from the `modifies` contract instrumentation by @feliperodri in https://github.com/model-checking/kani/pull/3169
* Emit source locations whenever possible to ease debugging and coverage reporting by @tautschnig in https://github.com/model-checking/kani/pull/3173
* Rust toolchain upgraded to `nightly-2024-04-21` by @celinval


**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.50.0...kani-0.51.0


## [0.50.0]

### Major Changes
* Fix compilation issue with proc_macro2  (v1.0.80+) and Kani v0.49.0
(https://github.com/model-checking/kani/issues/3138).

### What's Changed
* Implement valid value check for `write_bytes` by @celinval in
https://github.com/model-checking/kani/pull/3108
* Rust toolchain upgraded to 2024-04-15 by @tautschnig @celinval

**Full Changelog**:
https://github.com/model-checking/kani/compare/kani-0.49.0...kani-0.50.0

## [0.49.0]

### What's Changed
* Disable removal of storage markers by @zhassan-aws in https://github.com/model-checking/kani/pull/3083
* Ensure storage markers are kept in std code by @zhassan-aws in https://github.com/model-checking/kani/pull/3080
* Implement validity checks by @celinval in https://github.com/model-checking/kani/pull/3085
* Allow modifies clause for verification only by @feliperodri in https://github.com/model-checking/kani/pull/3098
* Add optional scatterplot to benchcomp output by @tautschnig in https://github.com/model-checking/kani/pull/3077
* Expand ${var} in benchcomp variant `env` by @karkhaz in https://github.com/model-checking/kani/pull/3090
* Add `benchcomp filter` command by @karkhaz in https://github.com/model-checking/kani/pull/3105
* Upgrade Rust toolchain to 2024-03-29 by @zhassan-aws @celinval @adpaco-aws @feliperodri

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.48.0...kani-0.49.0

## [0.48.0]

### Major Changes
* We fixed a soundness bug that in some cases may cause Kani to not detect a use-after-free issue in https://github.com/model-checking/kani/pull/3063

### What's Changed
* Fix `codegen_atomic_binop` for `atomic_ptr` by @qinheping in https://github.com/model-checking/kani/pull/3047
* Retrieve info for recursion tracker reliably by @feliperodri in https://github.com/model-checking/kani/pull/3045
* Add `--use-local-toolchain` to Kani setup by @jaisnan in https://github.com/model-checking/kani/pull/3056
* Replace internal reverse_postorder by a stable one by @celinval in https://github.com/model-checking/kani/pull/3064
* Add option to override `--crate-name` from `kani` by @adpaco-aws in https://github.com/model-checking/kani/pull/3054
* Rust toolchain upgraded to 2024-03-11 by @adpaco-ws @celinval @zyadh

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.47.0...kani-0.48.0

## [0.47.0]

### What's Changed
* Upgrade toolchain to 2024-02-14 by @zhassan-aws in https://github.com/model-checking/kani/pull/3036

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.46.0...kani-0.47.0

## [0.46.0]

## What's Changed
* `modifies` Clauses for Function Contracts by @JustusAdam in https://github.com/model-checking/kani/pull/2800
* Fix ICEs due to mismatched arguments by @celinval in https://github.com/model-checking/kani/pull/2994. Resolves the following issues:
  * https://github.com/model-checking/kani/issues/2260
  * https://github.com/model-checking/kani/issues/2312
* Enable powf*, exp*, log* intrinsics by @tautschnig in https://github.com/model-checking/kani/pull/2996
* Upgrade Rust toolchain to nightly-2024-01-24 by @celinval @feliperodri @qinheping

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.45.0...kani-0.46.0

## [0.45.0]

## What's Changed
* Upgrade toolchain to nightly-2024-01-17 by @celinval in https://github.com/model-checking/kani/pull/2976

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.44.0...kani-0.45.0

## [0.44.0]

### What's Changed

* Rust toolchain upgraded to `nightly-2024-01-08` by @adpaco-aws @celinval @zhassan-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.43.0...kani-0.44.0

## [0.43.0]

###  What's Changed
* Rust toolchain upgraded to `nightly-2023-12-14` by @tautschnig and @adpaco-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.42.0...kani-0.43.0

## [0.42.0]

### What's Changed

* Build CBMC from source and install as package on non-x86_64 by @bennofs in https://github.com/model-checking/kani/pull/2877 and https://github.com/model-checking/kani/pull/2878
* Emit suggestions and an explanation when CBMC runs out of memory by @JustusAdam in https://github.com/model-checking/kani/pull/2885
* Rust toolchain upgraded to `nightly-2023-11-28` by @celinval

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.41.0...kani-0.42.0

## [0.41.0]

### Breaking Changes

* Set minimum python to 3.7 in docker container and release action by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/2879
* Delete `any_slice` which has been deprecated since Kani 0.38.0. by @zhassan-aws in https://github.com/model-checking/kani/pull/2860

### What's Changed

* Make `cover` const by @jswrenn in https://github.com/model-checking/kani/pull/2867
* Change `expect()` from taking formatted strings to use `unwrap_or_else()` by @matthiaskrgr in https://github.com/model-checking/kani/pull/2865
* Fix setup for `aarch64-unknown-linux-gnu` platform by @adpaco-aws in https://github.com/model-checking/kani/pull/2864
* Do not override `std` library during playback by @celinval in https://github.com/model-checking/kani/pull/2852
* Rust toolchain upgraded to `nightly-2023-11-11` by @zhassan-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.40.0...kani-0.41.0

## [0.40.0]

### What's Changed

* Ease setup in Amazon Linux 2 by @adpaco-aws in https://github.com/model-checking/kani/pull/2833
* Propagate backend options into goto-synthesizer by @qinheping in https://github.com/model-checking/kani/pull/2643
* Update CBMC version to 5.95.1 by @adpaco-aws in https://github.com/model-checking/kani/pull/2844
* Rust toolchain upgraded to `nightly-2023-10-31` by @jaisnan @adpaco-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.39.0...kani-0.40.0

## [0.39.0]

### What's Changed

* Limit --exclude to workspace packages by @tautschnig in https://github.com/model-checking/kani/pull/2808
* Fix panic warning and add arbitrary Duration by @celinval in https://github.com/model-checking/kani/pull/2820
* Update CBMC version to 5.94 by @celinval in https://github.com/model-checking/kani/pull/2821
* Rust toolchain upgraded to `nightly-2023-10-17` by @celinval @tautschnig

**Full Changelog**:
https://github.com/model-checking/kani/compare/kani-0.38.0...kani-0.39.0

## [0.38.0]

### Major Changes

* Deprecate `any_slice` by @zhassan-aws in https://github.com/model-checking/kani/pull/2789

### What's Changed

* Provide better error message for invalid stubs by @JustusAdam in https://github.com/model-checking/kani/pull/2787
* Simple Stubbing with Contracts by @JustusAdam in https://github.com/model-checking/kani/pull/2746
* Avoid mismatch when generating structs that represent scalar data but also include ZSTs by @adpaco-aws in https://github.com/model-checking/kani/pull/2794
* Prevent kani crash during setup for first time by @jaisnan in https://github.com/model-checking/kani/pull/2799
* Create concrete playback temp files in source directory by @tautschnig in https://github.com/model-checking/kani/pull/2804
* Bump CBMC version by @zhassan-aws in https://github.com/model-checking/kani/pull/2796
* Update Rust toolchain to 2023-09-23 by @tautschnig in https://github.com/model-checking/kani/pull/2806

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.37.0...kani-0.38.0

## [0.37.0]

### Major Changes

* Delete obsolete stubs for `Vec` and related options by @zhassan-aws in https://github.com/model-checking/kani/pull/2770
* Add support for the ARM64 Linux platform by @adpaco-aws in https://github.com/model-checking/kani/pull/2757

### What's Changed

* Function Contracts: Support for defining and checking `requires` and `ensures` clauses by @JustusAdam in https://github.com/model-checking/kani/pull/2655
* Force `any_vec` capacity to match length by @celinval in https://github.com/model-checking/kani/pull/2765
* Fix expected value for `pref_align_of` under aarch64/macos by @remi-delmas-3000 in https://github.com/model-checking/kani/pull/2782
* Bump CBMC version to 5.92.0 by @zhassan-aws in https://github.com/model-checking/kani/pull/2771
* Upgrade to Kissat 3.1.1 by @zhassan-aws in https://github.com/model-checking/kani/pull/2756
* Rust toolchain upgraded to `nightly-2023-09-19` by @remi-delmas-3000 @tautschnig

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.36.0...kani-0.37.0

## [0.36.0]

### What's Changed

* Enable `-Z stubbing` and error out instead of ignoring stub by @celinval in https://github.com/model-checking/kani/pull/2678
* Enable concrete playback for failure of UB checks by @zhassan-aws in https://github.com/model-checking/kani/pull/2727
* Bump CBMC version to 5.91.0 by @adpaco-aws in https://github.com/model-checking/kani/pull/2733
* Rust toolchain upgraded to `nightly-2023-09-06` by @celinval @jaisnan @adpaco-aws

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.35.0...kani-0.36.0

## [0.35.0]

### What's Changed

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

### What's Changed

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

### What's Changed
* Add support for sysconf by @feliperodri in https://github.com/model-checking/kani/pull/2557
* Print Kani version by @adpaco-aws in https://github.com/model-checking/kani/pull/2619
* Upgrade Rust toolchain to nightly-2023-07-01 by @qinheping in https://github.com/model-checking/kani/pull/2616
* Bump CBMC version to 5.88.1 by @zhassan-aws in https://github.com/model-checking/kani/pull/2623

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.32.0...kani-0.33.0

## [0.32.0]

### What's Changed

* Add kani::spawn and an executor to the Kani library by @fzaiser in https://github.com/model-checking/kani/pull/1659
* Add "kani" configuration key to enable conditional compilation in build scripts by @celinval in https://github.com/model-checking/kani/pull/2297
* Adds posix_memalign to the list of supported builtins by @feliperodri in https://github.com/model-checking/kani/pull/2601
* Upgrade rust toolchain to nightly-2023-06-20 by @celinval in https://github.com/model-checking/kani/pull/2551
* Update rust toolchain to 2023-06-22 by @celinval in https://github.com/model-checking/kani/pull/2588
* Automatic toolchain upgrade to nightly-2023-06-24 by @github-actions in https://github.com/model-checking/kani/pull/2600
* Bump CBMC version to 5.87.0 by @adpaco-aws in https://github.com/model-checking/kani/pull/2598

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.31.0...kani-0.32.0

## [0.31.0]

### What's Changed
* Add `--exact` flag by @jaisnan in https://github.com/model-checking/kani/pull/2527
* Build the verification libraries using Kani compiler by @celinval in https://github.com/model-checking/kani/pull/2534
* Verify all Kani attributes in all crate items upfront by @celinval in https://github.com/model-checking/kani/pull/2536
* Update README.md - fix link locations for badge images in markdown by @phayes in https://github.com/model-checking/kani/pull/2537
* Bump CBMC version to 5.86.0 by @zhassan-aws in https://github.com/model-checking/kani/pull/2561

**Full Changelog**: https://github.com/model-checking/kani/compare/kani-0.30.0...kani-0.31.0

## [0.30.0]

### What's Changed
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
