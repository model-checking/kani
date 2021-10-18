#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# ANCHOR: cargo
set -eux

# The argument to this script will be the proof harness, e.g. 'my_harness' above
ENTRY_POINT=$1

export CARGO_TARGET_DIR=target-rmc

env \
  RUST_BACKTRACE=1 \
  RUSTC=rmc-rustc \
  RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" \
  cargo build --target x86_64-unknown-linux-gnu
# ANCHOR_END: cargo


# ANCHOR: linking
cd $CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/debug/deps

# Independently translate each symbol table into a goto binary
ls *.json | parallel symtab2gb {} --out {.}.out

# Link everything together
goto-cc *.out -o linked-binary.out
# ANCHOR_END: linking


# TEMPORARY FIX
# Empty C file so CBMC inserts its header
touch empty.c
# without this, we get cbmc errors about __CPROVER_dead_object missing


# ANCHOR: cbmc
# Now for each harness we specialize a binary:
HARNESS_BIN="harness_${ENTRY_POINT}.out"
goto-cc --function ${ENTRY_POINT} linked-binary.out empty.c -o "${HARNESS_BIN}"

# Perform some preprocessing
INSTRUMENT_ARGS=(
  --drop-unused-functions
)
goto-instrument "${INSTRUMENT_ARGS[@]}" "${HARNESS_BIN}" "${HARNESS_BIN}"

# Run CBMC, passing along appropriate CBMC arguments:
CBMC_ARGS=(
  # RMC defaults
  --unwinding-assertions
  --bounds-check
  --pointer-check
  --pointer-primitive-check
  --pointer-overflow-check
  --signed-overflow-check
  --undefined-shift-check
  --unsigned-overflow-check
  --conversion-check
  --div-by-zero-check
  --float-overflow-check
  --nan-check
  # Additional options
  --unwind 0
  --object-bits 11
)

cbmc "${CBMC_ARGS[@]}" "${HARNESS_BIN}"
# ANCHOR_END: cbmc
