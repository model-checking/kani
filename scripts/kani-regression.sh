#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $KANI_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH
EXTRA_X_PY_BUILD_ARGS="${EXTRA_X_PY_BUILD_ARGS:-}"
KANI_DIR=$SCRIPT_DIR/..

# This variable forces an error when there is a mismatch on the expected
# descriptions from cbmc checks.
# TODO: We should add a more robust mechanism to detect python unexpected behavior.
export KANI_FAIL_ON_UNEXPECTED_DESCRIPTION="true"

# Gather dependencies version from top `kani-dependencies` file.
source "${KANI_DIR}/kani-dependencies"
# Sanity check dependencies values.
[[ "${CBMC_MAJOR}.${CBMC_MINOR}" == "${CBMC_VERSION%.*}" ]] || \
    (echo "Conflicting CBMC versions"; exit 1)
# Check if installed versions are correct.
check-cbmc-version.py --major ${CBMC_MAJOR} --minor ${CBMC_MINOR}
check_kissat_version.sh

# Formatting check
${SCRIPT_DIR}/kani-fmt.sh --check

# Build kani
cargo build-dev

# Unit tests
cargo test -p cprover_bindings
cargo test -p kani-compiler
cargo test -p kani-driver
cargo test -p kani_metadata
# Use concrete playback to enable assertions failure
cargo test -p kani --features concrete_playback
# Test the actual macros, skipping doc tests and enabling extra traits for "syn"
# so we can debug print AST
RUSTFLAGS=--cfg=kani_sysroot cargo test -p kani_macros --features syn/extra-traits --lib

# Declare testing suite information (suite and mode)
TESTS=(
    "kani kani"
    "expected expected"
    "ui expected"
    "std-checks cargo-kani"
    "firecracker kani"
    "prusti kani"
    "smack kani"
    "cargo-kani cargo-kani"
    "cargo-ui cargo-kani"
    "script-based-pre exec"
    "coverage coverage-based"
    "cargo-coverage cargo-coverage"
    "kani-docs cargo-kani"
    "kani-fixme kani-fixme"
)

# Build compiletest and print configuration. We pick suite / mode combo so there's no test.
echo "--- Compiletest configuration"
cargo run -p compiletest --quiet -- --suite kani --mode cargo-kani --dry-run --verbose
echo "-----------------------------"

# Build `kani-cov`
cargo build -p kani-cov

# Extract testing suite information and run compiletest
# Bound each test's wall time so a hanging test (e.g. one whose verification
# fails to terminate under a new toolchain) fails as a normal test failure with
# output, instead of stalling the whole run until the CI job is killed hours
# later. The nightly toolchain-upgrade automation relies on this: a clean
# regression failure lets it open a tracking issue, whereas an unbounded hang
# produces an unattributable job timeout. The cap is a guardrail against hangs,
# not a tight performance bound, so it sits well above the slowest legitimate
# test. The slowest single test is expected/shadow/unsupported_num_objects,
# whose ~1020-object loops can take ~15 min on the slowest CI runners, so the
# default is set with comfortable margin above that. Override via
# KANI_REGRESSION_TEST_TIMEOUT (a positive integer number of seconds).
timeout_secs="${KANI_REGRESSION_TEST_TIMEOUT:-2400}"
if ! [[ "$timeout_secs" =~ ^[1-9][0-9]*$ ]]; then
  echo "error: KANI_REGRESSION_TEST_TIMEOUT must be a positive integer (seconds), got '$timeout_secs'" >&2
  exit 1
fi
for testp in "${TESTS[@]}"; do
  testl=($testp)
  suite=${testl[0]}
  mode=${testl[1]}
  echo "Check compiletest suite=$suite mode=$mode timeout=${timeout_secs}s"
  cargo run -p compiletest --quiet -- --suite $suite --mode $mode \
      --quiet --no-fail-fast --timeout "$timeout_secs"
done

# We rarely benefit from re-using build artifacts in the firecracker test,
# and we often end up with incompatible leftover artifacts:
# "error[E0514]: found crate `serde_derive` compiled by an incompatible version of rustc"
# So if we're calling the full regression suite, wipe out old artifacts.
if [ -d "$KANI_DIR/firecracker/build" ]; then
  rm -rf "$KANI_DIR/firecracker/build"
fi

# Check codegen of firecracker
time "$SCRIPT_DIR"/codegen-firecracker.sh

# Test for --manifest-path which we cannot do through compiletest.
# It should just successfully find the project and specified proof harness. (Then clean up.)
FEATURES_MANIFEST_PATH="$KANI_DIR/tests/cargo-kani/cargo-features-flag/Cargo.toml"
cargo kani --manifest-path "$FEATURES_MANIFEST_PATH" --harness trivial_success
cargo clean --manifest-path "$FEATURES_MANIFEST_PATH"

# Build all packages in the workspace and ensure no warning is emitted.
# Please don't replace `cargo build-dev` above with this command.
# Setting RUSTFLAGS like this always resets cargo's build cache resulting in
# all tests to be re-run. I.e., cannot keep re-runing the regression from where
# we stopped.
# Only run with the `cprover` feature to avoid compiling the `charon` library
# which is not our code and may have warnings. The downside is that we wouldn't
# detect any warnings in the charon code path. TODO: Remove
# `--no-default-features --features cprover` when the warnings in charon are
# fixed and we advance the charon pin to that version
RUSTFLAGS="-D warnings" cargo build --target-dir /tmp/kani_build_warnings --no-default-features --features cprover

echo
echo "All Kani regression tests completed successfully."
echo
