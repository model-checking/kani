#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

echo "[without flag]"
# Kani's macro-override injection makes `assert!` ambiguous, so `cargo kani`
# must fail with rustc's E0659 (GlobVsOuter). Capture the status explicitly:
# the previous `cargo kani ... | grep E0659` pipeline reported grep's status,
# so a change in the driver's exit behavior or a stray E0659 elsewhere could
# have silently made the test pass for the wrong reason.
status=0
output=$(cargo kani 2>&1) || status=$?
if [ "$status" -eq 0 ]; then
    echo "unexpected success: cargo kani should have failed with an ambiguity error"
    echo "$output"
    exit 1
fi
# Match the specific ambiguity error rather than a bare `E0659`, so the
# `.expected` check confirms the first run failed for the intended reason.
if ! grep -m1 -F 'error[E0659]: `assert` is ambiguous' <<<"$output"; then
    echo "expected E0659 ambiguity error not found in cargo kani output; got:"
    echo "$output"
    exit 1
fi

echo "[with flag]"
# The macro-override injection is skipped, so the crate compiles. The
# intentionally failing harness still fails verification, producing the
# expected non-zero exit status (see config.yml: exit_code: 1).
cargo kani --no-assert-overrides
