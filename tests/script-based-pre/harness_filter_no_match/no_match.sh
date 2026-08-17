#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Regression test for https://github.com/model-checking/kani/issues/4731:
# a `--harness` filter that matches no harness must fail (non-zero exit + an
# error) *before* codegen/export -- even under `--quiet`, which suppresses the
# end-of-run summary where the "no harnesses matched" error is otherwise raised.
# Previously such a run exited 0 and, with `--export-json`, wrote a document
# that serialized an empty run as a clean pass.

set +e

# 1. Under `--quiet`, a non-matching filter must still fail with an error.
OUT=$(kani fixture.rs --quiet --harness does_not_exist 2>&1)
CODE=$?
if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: zero-match run under --quiet exited 0"
    exit 1
fi
if ! grep -q "no harnesses matched" <<< "${OUT}"; then
    echo "FAIL: expected a 'no harnesses matched' error, got:"
    echo "${OUT}"
    exit 1
fi

# 2. With `--export-json`, a non-matching filter must fail and must not write a
#    document (the run must fail before the export is written).
rm -f out.json
kani fixture.rs -Z unstable-options --harness does_not_exist --export-json out.json >/dev/null 2>&1
CODE=$?
if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: zero-match run with --export-json exited 0"
    exit 1
fi
if [[ -f out.json ]]; then
    echo "FAIL: --export-json wrote a document for a zero-match run"
    rm -f out.json
    exit 1
fi

# 3. Several non-matching filters must all be named in the error.
OUT=$(kani fixture.rs --harness does_not_exist_a --harness does_not_exist_b 2>&1)
CODE=$?
if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: zero-match run with two filters exited 0"
    exit 1
fi
if ! grep -q 'no harnesses matched the harness filters: `does_not_exist_a`, `does_not_exist_b`' <<< "${OUT}"; then
    echo "FAIL: expected both filters in the error, got:"
    echo "${OUT}"
    exit 1
fi

echo "SUCCESS: zero-match filters fail before exit and before export"
