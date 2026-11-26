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

# A project that pins a toolchain rustup cannot install: the plugin must not need it (and must
# never try to install it), while plain `cargo kani` fails.
printf '[toolchain]\nchannel = "kani-plugin-no-such-toolchain"\n' > "$FIXTURE/rust-toolchain.toml"
if (cd "$FIXTURE" && cargo kani --version > /dev/null 2>&1); then
    echo "note: rustup is not in use; the toolchain pin check is vacuous here"
fi
"$SCRIPTS/check-specs.py" "$FIXTURE/src" --function checked_div > "$WORK/pinned.json" 2>&1 \
    || [ $? -eq 2 ] || fail "check-specs.py failed in a project pinning a missing toolchain: $(cat "$WORK/pinned.json")"
grep -q requires-satisfiable "$WORK/pinned.json" || fail "check-specs.py did not run: $(cat "$WORK/pinned.json")"
build_mcp
"$PLUGIN/tests/mcp_client.py" "$KANI_MCP" kani_list_harnesses \
    "{\"path\": \"$FIXTURE\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" > "$WORK/pinned-mcp.json"
[ "$(jq_py "$WORK/pinned-mcp.json" 'd["is_error"]')" = "False" ] || fail "kani-mcp failed: $(cat "$WORK/pinned-mcp.json")"
if command -v rustup > /dev/null && rustup toolchain list | grep -q kani-plugin-no-such-toolchain; then
    fail "a toolchain was installed"
fi
echo "the scripts and kani-mcp run in a project that pins a toolchain, without installing it"
smoke_pass
