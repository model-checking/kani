# RMC Developer Guide

## Building RMC
1. Follow [the quickstart instructions](README.md#quickstart) to initially build RMC.
1. Subsequent builds can be done incrementally, using 
   ```
   ./x.py build -i --stage 1 library/std --keep-stage 1
   ```
1. If you encounter unexpected issues, do a full rebuild
   ```
   ./x.py clean
   ./x.py build -i --stage 1 library/std
   ```

## Submitting / updating PRs
1. Ensure that your changes are rebased against the latest `main-<upstream-version>-yyyy-mm-dd`.
   If your PR is open across a `main*` branch update, we recommend doing a rebase and force push.
1. Ensure that your code is properly formatted.
   ```
   ./x.py fmt
   ```
1. Ensure that all regressions pass
   ```
   ./scripts/rmc-regression.sh
   ```

## Architecture
TODO

## Branch structure
RMC is implemented as an additional codegen backend for the 
[Rust compiler](https://github.com/rust-lang/rust).
The `master` branch from the upstream compiler is mirrored as `upstream-master`.
The RMC code itself is maintained as a set of rebased patches, 
    on branches named `main-<upstream-version>-yyyy-mm-dd`.
These branches are updated on a weekly cadence.
The most recent of these branches is set as the `default` branch for the repository.
This is the branch that you should build and use, and this is the branch that you should make PRs against.

### Patch structure
The `main-<upstream-version>-yyyy-mm-dd` branches have the following git structure:

* The upstream `master` branch as of the date `yyyy-mm-dd`.
* A source code patch that makes all changes to the upstream code needed for RMC to link.
* A renaming patch that renames upstream files that conflict with RMC files.
* A set of commits representing RMC feature code.
