# `cargo kani assess`

Assess is an experimental new feature to gather data about Rust crates, to aid the start of proof writing.

In the short-term, assess collects and dumps tables of data that may help _Kani developers_ understand what's needed to begin writing proofs for another project.
For instance, assess may help answer questions like:

1. Does Kani successfully build all of the crates involved in this project? If not, why not?
2. Does Kani support all the Rust language features necessary to do verification with this project? If not, which are most important?

In the long-term, assess will become a user-facing feature, and help _Kani users_ get started writing proofs.
We expect that users will have the same questions as above, but in the long term, hopefully the answers to those trend towards an uninteresting "yes."
So the new questions might be:

3. Is this project ready for verification? Projects need to be reasonably well-tested first.
Our operating hypothesis is that code currently covered by unit tests is the code that could become covered by proofs.
4. How much of given project (consisting of multiple packages or workspaces) or which of the user's projects might be verifiable?
If a user wants to start trying Kani, but they have the choice of several different packages where they might try, we can help find the package with the lowest hanging fruit.
5. Given a package, where in that package's code should the user look, in order to write the first (or next) proof?

These long-term goals are only "hinted at" with the present experimental version of assess.
Currently, we only get as far as finding out which tests successfully verify (concretely) with Kani.
This might indicate tests that could be generalized and converted into proofs, but we currently don't do anything to group, rank, or otherwise heuristically prioritize what might be most "interesting."
(For instance, we'd like to eventually compute coverage information and use that to help rank the results.)
As a consequence, the output of the tool is very hard to interpret, and likely not (yet!) helpful to new or potential Kani users.

## Using Assess

To assess a package, run:

```text
cargo kani --enable-unstable assess
```

As a temporary hack (arguments shouldn't work like this), to assess a single cargo workspace, run:

```text
cargo kani --enable-unstable --workspace assess
```

To scan a collection of workspaces or packages that are not part of a shared workspace, run:

```text
cargo kani --enable-unstable assess scan
```

The only difference between 'scan' and 'regular' assess is how the packages built are located.
All versions of assess produce the same output and metrics.
Assess will normally build just like `cargo kani` or `cargo build`, whereas `scan` will find all cargo packages beneath the current directory, even in unrelated workspaces.
Thus, 'scan' may be helpful in the case where the user has a choice of packages and is looking for the easiest to get started with (in addition to the Kani developer use-case, of aggregating statistics across many packages).

(Tip: Assess may need to run for awhile, so try using `screen`, `tmux` or `nohup` to avoid terminating the process if, for example, an ssh connection breaks.)

## What assess does

Assess builds all the packages requested in "test mode" (i.e. `--tests`), and runs all the same tests that `cargo test` would, except through Kani.
This gives end-to-end assurance we're able to actually build and run code from these packages, skipping nothing of what the verification process would need, except that the harnesses don't have any nondeterminism (`kani::any()`) and consequently don't "prove" much.
The interesting signal comes from what tests cannot be analyzed by Kani due to unsupported features, performance problems, crash bugs, or other issues that get in the way.

Currently, assess forces termination by using `unwind(1)` on all tests, so many tests will fail with unwinding assertions.

## Current Assess Results

Assess produces a few tables of output (both visually in the terminal, and in a more detailed json format) so far:

### Unsupported features

```text
======================================================
 Unsupported feature           |   Crates | Instances
                               | impacted |    of use
-------------------------------+----------+-----------
 caller_location               |       71 |       239
 simd_bitmask                  |       39 |       160
...
```

The unsupported features table aggregates information about features that Kani does not yet support.
These correspond to uses of `codegen_unimplemented` in the `kani-compiler`, and appear as warnings during compilation.

Unimplemented features are not necessarily actually hit by (dynamically) reachable code, so an immediate future improvement on this table would be to count the features *actually hit* by failing test cases, instead of just those features reported as existing in code by the compiler.
In other words, the current unsupported features table is **not** what we'd really want to see, in order to actually prioritize implementing these features, because we may be seeing a lot of features that won't actually "move the needle" in making it more practical to write proofs.
Because of our operating hypothesis that code covered by tests is code that could be covered by proof, measuring unsupported features by those actually hit by a test should provide a better "signal" about priorities.
Implicitly deprioritizing unsupported features because they aren't covered by tests may not be a bug, but a feature: we may simply not want to prove anything about that code, if it hasn't been tested first, and so adding support for that feature may not be important.

A few notes on terminology:

1. "Crates impacted" here means "packages in the current workspace (or scan) where the building of that package (and all of its dependencies) ultimately resulted in this warning."
For example, if only assessing a single package (not a workspace) this could only be `1` in this column, regardless of the number of dependencies.
2. "Instances of use" likewise means "total instances found while compiling this package's tests and all the (reachable) code in its dependencies."
3. These counts are influenced by (static) reachability: if code is not potentially reachable from a test somehow, it will not be built and will not be counted.

### Test failure reasons

```text
================================================
 Reason for failure           | Number of tests
------------------------------+-----------------
 unwind                       |              61
 none (success)               |               6
 assertion + overflow         |               2
...
```

The test failure reasons table indicates why, when assess ran a test through Kani, it failed to verify.
Notably:

1. Because we force termination with `unwind(1)`, we expect `unwind` to rank highly.
2. We do report number of tests succeeding on this table, to aid understanding how well things went overall.
3. The reported reason is the "property class" of the CBMC property that failed. So `assertion` means an ordinary `assert!()` was hit (or something else with this property class).
4. When multiple properties fail, they are aggregated with `+`, such as `assertion + overflow`.
5. Currently this table does not properly account for `should_fail` tests, so `assertion` may actually be "success": the test should hit an assertion and did.

### Promising test cases

```text
=============================================================================
 Candidate for proof harness                           | Location
-------------------------------------------------------+---------------------
 float::tests::f64_edge_cases                          | src/float.rs:226
 float::tests::f32_edge_cases                          | src/float.rs:184
 integer::tests::test_integers                         | src/integer.rs:171
```

This table is the most rudimentary so far, but is the core of what long-term assess will help accomplish.
Currently, this table just presents (with paths displayed in a clickable manner) the tests that successfully "verify" with Kani.
These might be good candidates for turning into proof harnesses.
This list is presently unordered; the next step for improving it would be to find even a rudimentary way of ranking these test cases (e.g. perhaps by code coverage).

## How Assess Works

`kani-compiler` emits `*.kani-metadata.json` for each target it builds.
This format can be found in the `kani_metadata` crate, shared by `kani-compiler` and `kani-driver`.
This is the starting point for assess.

Assess obtains this metadata by essentially running a `cargo kani`:

1. With `--all-features` turned on
2. With `unwind` always set to `1`
3. In test mode, i.e. `--tests`
4. With test-case reachability mode. Normally Kani looks for proof harnesses and builds only those. Here we switch to building only the test harnesses instead.

Assess starts by getting all the information from these metadata files.
This is enough by itself to construct a rudimentary "unsupported features" table.
But assess also uses it to discover all the test cases, and (instead of running proof harnesses) it then runs all these test harnesses under Kani.

Assess produces a second metadata format, called (unsurprisingly) "assess metadata".
(Found in `kani-driver` under [`src/assess/metadata.rs`](https://github.com/model-checking/kani/blob/main/kani-driver/src/assess/metadata.rs).)
This format records the results of what assess does.

This metadata can be written to a json file by providing `--emit-metadata <file>` to `assess`.
Likewise, `scan` can be told to write out this data with the same option.

Assess metadata is an aggregatable format.
It does not apply to just one package, as assess can work on a workspace of packages.
Likewise, `scan` uses and produces the exact same format, across multiple workspaces.

So far all assess metadata comes in the form of "tables" which are built with `TableBuilder<T: TableRow>`.
This is documented further in [`src/assess/table_builder.rs`](https://github.com/model-checking/kani/blob/main/kani-driver/src/assess/table_builder.rs).
