- **Feature Name:** Viewerless coverage information (`viewerless-coverage`)
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/2610>
- **RFC PR:** <https://github.com/model-checking/kani/pull/2609>
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** <https://github.com/model-checking/kani/pull/2609>

-------------------

## Summary

An option in Kani to generate coverage reports without `cbmc-viewer`.

## User Impact

Kani has relied on [`cbmc-viewer`](https://github.com/model-checking/cbmc-viewer) to report coverage information since the beginning.
In essence, `cbmc-viewer` consumes data from coverage-oriented invocations of CBMC and produces an HTML report containing (1) coverage information and (2) counterexample traces.
Recently, there have been some issues with the coverage information reported by `cbmc-viewer` (e.g., [#2048](https://github.com/model-checking/kani/issues/2048) or [#1707](https://github.com/model-checking/kani/issues/1707)), forcing us to mark the `--visualize` option as unstable and disable coverage results in the reports (in [#2206](https://github.com/model-checking/kani/pull/2206)).

However, it is possible for Kani to report coverage information without `cbmc-viewer`.
This only requires a new option in Kani that enables the injection of coverage-oriented checks (similar to `kani::cover` statements) into certain parts of the program under verification.
The status reported by the injected checks is sufficient to construct a coverage report.
Note that this gives Kani control on both ends:
 * **The instrumentation performed** on the program. Eventually, this would allow us to report more precise coverage information, similar to [Rust's instrument-based code coverage](https://doc.rust-lang.org/rustc/instrument-coverage.html).
 * **The format of the coverage report** to be generated. Similarly, this would allow us to generate coverage data in different formats (see [#1706](https://github.com/model-checking/kani/issues/1706) for GCOV, or [#1777](https://github.com/model-checking/kani/issues/1777) for LCOV). While technically this is also doable from `cbmc-viewer`'s output, development is likely to be faster this way.

Moreover, producing coverage information directly from Kani is likely to increase speed of development and improve testing for coverage features, which translates into faster and more reliable coverage options for users.

## User Experience

For the first version, this experimental feature will be limited to produce a sequence of text lines as follows:
```
<file>, <line>, <status>
```
where `<status>` is either `COVERED` or `UNCOVERED`.

**Users are not expected to consume this output directly.**
Instead, coverage data is to be consumed by the [Kani VS Code extension](https://github.com/model-checking/kani-vscode-extension) and displayed as in the following picture:

![Coverage reported on the Kani VS Code Extension](../images/0008/coverage-kani-vscode.png)

How to activate and display coverage information in the extension is out of scope for this RFC.
We will need to change our documentation (including the tutorial) to use this option if we ever remove `--visualize` in favor of this one.

## Detailed Design

We will add a new unstable `--coverage` verification option to Kani.
This will prevent `kani-driver` from reporting verification results as usual and instead output the coverage information described above[^coverage-assertions].
It's likely that this will be done through a new output format that's not exposed to users.

We will also add a new `--coverage-checks` option to `kani-compiler`, which will result in the injection of coverage checks before each Rust statement and terminator[^coverage-experiments].
This option will be supplied by `kani-driver` when the `--coverage` option is selected.
These coverage checks can be processed after regular verification postprocessing to obtain the coverage information, which can directly be passed on to the Kani VS Code Extension.

Eventually, this could allow us to remove `cbmc-viewer` as a dependency.

We anticipate missing locations being the main corner case for the coverage feature.
However, coverage reported through `cbmc-viewer` also suffers from this issue.
This will be mitigated by ignoring checks coming from unknown locations and propagating locations if possible whenever they're missing (as suggested in [this comment](https://github.com/model-checking/kani/issues/2048#issuecomment-1599680694)).

## Rationale and alternatives

The main advantage of this design is that it'll give Kani full control over its coverage-related options.
This implies that not only it'll be able to select the mechanism to collect coverage information, but also have different options to output coverage data.
All of that fully integrated in Kani and tested through `compiletest`.
In contrast, this means that Kani will become more complex, so the development and maintainance burden will be higher.

This feature is expected to have a substantial impact for Kani.
Users frequently ask for coverage-related options but Kani doesn't have one.
That's because, some months ago, we noticed incorrect coverage results were being reported with the `--visualize` option, prompting us to disable the coverage reports emitted by `cbmc-viewer`.

## Open questions

Open questions:
 * Do we want a `--coverage` option or a `coverage` subcommand? Coverage can be seen as a different workflow but also a verification option.

## Future possibilities

We expect many incremental improvements in the coverage area in subsequent versions:
 1. Replacing the injection mechanism proposed in this RFC with the Rust compiler APIs for coverage (e.g., [CodeRegion](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/coverage/struct.CodeRegion.html) and/or a MIR pass similar to [InstrumentCoverage](https://doc.rust-lang.org/stable/nightly-rustc/rustc_mir_transform/coverage/struct.InstrumentCoverage.html)) so we can retrieve region-based coverage information. Note that this is a requirement for items (2) and (3) below.
 2. Displaying region-based coverage information similar to [Rust's instrument-based code coverage](https://doc.rust-lang.org/rustc/instrument-coverage.html), which allows users to see coverage information for more granular parts of code (e.g., subconditions hit in a disjunctive condition).
 3. Adding new user-requested coverage formats such as GCOV [#1706](https://github.com/model-checking/kani/issues/1706) or LCOV [#1777](https://github.com/model-checking/kani/issues/1777).
 4. Enabling an option to run verification and coverage at the same time, so users can obtain both results at the same time.
 5. Performing optimization improvements to `kani-compiler` and its engines to speed up the data collected for coverage information.


[^coverage-assertions]: Currently, we replace non-coverage assertions with assumptions so the execution is blocked instead of reporting a failure.
Because of that, we cannot report coverage runs as verification results.

[^coverage-experiments]: We have experimented with different options for injecting coverage checks.
For example, we have tried injecting one before each basic block, or one before each statement, etc.
The proposed option (one before each statement AND each terminator) gives us the most accurate results.
