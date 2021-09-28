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
1. Ensure that your changes are rebased against `main`.
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
Changes from the upstream compiler are merged with RMC's `main` branch on a weekly cadence.
