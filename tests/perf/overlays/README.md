# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

This directory contains "overlay" files (e.g. expected files) that should be copied into directories under perf before running compiletest.

Explanation: compiletest's cargo-kani mode (which is used for running the perf tests) looks for "<harness-name>.expected" files and runs `cargo kani --harness <harness-name>` for each.
Some of the perf tests are external repositories that are integrated as git submodules, so we cannot commit files in their subtrees.
Thus, we instead commit any files needed under the "overlays" directory, which then get copied over by `kani-perf.sh` before it calls compiletest.

To create overlay files for `perf/foo`, place them in a `perf/overlays/foo` directory.
They will get copied over following the same directory structure.
