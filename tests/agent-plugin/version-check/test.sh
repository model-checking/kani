#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The plugin detects the installed Kani and fails clearly on an unsupported version.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"

"$SCRIPTS/check-kani-version.py" --kani-cmd kani > "$WORK/version.json" \
    || fail "the Kani under test is not supported by the plugin: $(cat "$WORK/version.json")"
[ "$(jq_py "$WORK/version.json" 'd["compatible"]')" = "True" ] || fail "expected compatible"
echo "check-kani-version.py accepts the Kani under test"

if "$SCRIPTS/check-kani-version.py" --version-string "kani 0.1.0" > /dev/null 2> "$WORK/err.txt"; then
    fail "Kani 0.1.0 should be rejected"
fi
grep -q "older than the minimum supported version" "$WORK/err.txt" \
    || fail "unclear error for an old Kani: $(cat "$WORK/err.txt")"
echo "check-kani-version.py rejects an old Kani with a clear message"

if "$SCRIPTS/check-kani-version.py" --kani-cmd "$WORK/no-such-kani" > /dev/null 2> "$WORK/err.txt"; then
    fail "a missing Kani should be rejected"
fi
grep -q "cargo install --locked kani-verifier" "$WORK/err.txt" \
    || fail "missing install hint: $(cat "$WORK/err.txt")"
echo "check-kani-version.py reports a missing Kani with the install command"
