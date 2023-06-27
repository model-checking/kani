- **Feature Name:** Print Kani version (`kani-version`)
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/2570>
- **RFC PR:** <https://github.com/model-checking/kani/issues/2571>
- **Status:** Under Review
- **Version:** 0

-------------------

## Summary

Print the version of Kani at the beginning of a run.

## User Impact

Many programs print their version at the beginning of a run.
The version of a program communicates the state of the software at a given point (e.g., features that are available or performance on particular problems).
At present, Kani does not print its version, but it's something we should strongly consider from now on.

There are many benefits to including the version of Kani in its output.
However, I think the main ones will be the following:
 * **Earlier detection of version-related discrepancies**:
 Users are likely to discuss discrepancies in verification outcomes by looking at Kani's output.
 These may look exactly the same[^cbmc-version] (except for the discrepant value) on two different versions of Kani.
 Including the version will help users realize sooner that they're using different versions of Kani.
 * **Simpler issue triaging**:
 New issues require users to post the Kani version they used.
 Getting this information requires another call with `--version`, which wouldn't be needed if we simply printed the version.
 Also, note that users may need to do more work if they aren't running Kani locally (e.g., Kani running in CI).

In addition, printing the Kani version may be useful for other purposes (automate CI processes, help users realize they're using outdated versions, etc.).

## User Experience

The first line printed in any Kani invocation (either through `kani` or `cargo kani`, and regardless of subcommands) will inform users of the version.
The behavior will be extended for development versions, where it'll print the short hash of the HEAD commit in addition to the version.

### Release versions

The first line to be printed will be:

```
Launching the Kani Rust Verifier <version>
```

where `<version>` is the version of Kani under use, which follows the semantic versioning format `MAJOR.MINOR.PATCH`.

For example, for the release version of [Kani 0.29.0](https://github.com/model-checking/kani/releases/tag/kani-0.29.0), this would have printed:

```
Launching the Kani Rust Verifier 0.29.0
```

### Development versions

The first line to be printed will be:

```
Launching the Kani Rust Verifier <version> (dev. version - commit: <commit>)
```

where `<version>` is the version of Kani under use, which follows the semantic versioning format `MAJOR.MINOR.PATCH`,
and `<commit>` is the short hash (i.e., 7 hexadecimal digits with format `hhhhhhh`) of the `HEAD` commit.

For example, for the development version of [Kani 0.29.0](https://github.com/model-checking/kani/releases/tag/kani-0.29.0), this would have printed:

```
Launching the Kani Rust Verifier 0.29.0 (dev. version - commit: e4f989b)
```

## Detailed Design

The implementation will require additions to the `kani-driver` module.

Printing the short hash of the `HEAD` commit would require `git` as a dependency, but it can be made optional if we print `unknown` in the case where `git` isn't available.

## Rationale and alternatives

It's possible to argue that Kani shouldn't print its version because other (related) tools don't (e.g., `rustc`).
However, many of those tools are expected to NOT produce any output when all went well (i.e., no errors nor warnings when compiling a program).
This isn't something we expect Kani to do though: it'll always produce some output to inform users about the verification results.

In my experience, we should print the version because users and developers use text-based log files containing Kani's output to discuss verification results.
In some cases, we've had to "calculate" the Kani version from the CBMC versions appearing in the log.
But we shouldn't need to in the first place.

### Style alternatives

It'd be great to discuss any alternatives for the concrete format.
At some point, I even thought about adding some ASCII art, but wanted to keep it short and simple.

For example, we could:
 - Replace the word `Launching` with another one.
 - Prefix the version with `v` (so the version gets printed as `v0.29.0`, for example).
 - Just print `Kani Rust Verifier <version>`, nothing else.

These are low-level details which I'd love to discuss with you all.

## Open questions

I'm hoping that we can answer the following questions during the RFC:
 1. Do we want to print Kani's version?
 2. If we decide to move forward, what's your preferred style?

## Future possibilities

No future possibilities are under consideration.

[^cbmc-version]: The CBMC version is printed once for each harness.
That'd be the main difference between outputs from different versions, but only if the CBMC version was bumped in between.
