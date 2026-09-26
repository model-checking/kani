#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# An empty or whitespace-padded `RUSTFLAGS` used to reach rustc as an empty argument, which it
# reads as a second input filename: https://github.com/model-checking/kani/issues/4816

set -e
rm -rf target

RUSTFLAGS="" kani standalone.rs --output-format terse
echo "empty RUSTFLAGS works"

RUSTFLAGS="   " kani standalone.rs --output-format terse
echo "blank RUSTFLAGS works"

RUSTFLAGS="--cfg=first  --cfg=second" kani cfgs.rs --output-format terse
echo "repeated spaces keep both flags"

RUSTFLAGS="" cargo kani --output-format terse
echo "empty RUSTFLAGS works for cargo kani"

rm -rf target
