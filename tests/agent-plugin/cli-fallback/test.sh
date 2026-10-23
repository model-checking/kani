#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The full skill workflow without the MCP server: run Kani through the CLI with JSON export,
# summarize results, produce a concrete playback reproducer, check for vacuity and suspicious
# assumptions, and run AutoHarness (which has no MCP tool).

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"
cd "$FIXTURE"

# Verify all harnesses. One harness fails, so Kani exits non-zero.
if cargo kani -Z export-json --export-json "$WORK/all.json" > "$WORK/all.log" 2>&1; then
    fail "expected check_naive_midpoint to fail"
fi
[ -f "$WORK/all.json" ] || fail "no JSON export: $(tail -n 30 "$WORK/all.log")"

"$SCRIPTS/summarize-results.py" "$WORK/all.json" > "$WORK/summary.json"
python3 - "$WORK/summary.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
by_name = {h["name"].split("::")[-1]: h for h in s["harnesses"]}
assert s["total"] == 3 and s["failed"] == 1, s
assert by_name["check_midpoint"]["verdict"] == "SUCCESS", by_name
naive = by_name["check_naive_midpoint"]
assert naive["verdict"] == "FAILURE", naive
assert any("overflow" in f["description"] for f in naive["failed"]), naive
assert all(f["location"].startswith("src/lib.rs:") for f in naive["failed"]), naive
assert by_name["check_vacuous"]["verdict"] == "SUCCESS", by_name
# Compactness: the summary is a small fraction of the full export.
PY
[ "$(wc -c < "$WORK/summary.json")" -lt "$(( $(wc -c < "$WORK/all.json") / 3 ))" ] \
    || fail "summary is not compact"
echo "summarize-results.py reports the success, the overflow, and stays compact"

# The vacuous harness passes, but check-vacuity.py must flag it; the real proof must be clean.
set +e
"$SCRIPTS/check-vacuity.py" "$WORK/all.json" > "$WORK/vacuity.json"
status=$?
set -e
[ "$status" -eq 2 ] || fail "check-vacuity.py should exit 2, got $status"
python3 - "$WORK/vacuity.json" <<'PY'
import json, sys
findings = json.load(open(sys.argv[1]))["findings"]
blocking = {(f["harness"].split("::")[-1], f["rule"]) for f in findings if f["severity"] != "info"}
assert ("check_vacuous", "unreachable-assertion") in blocking, findings
assert not any(h == "check_midpoint" for h, _ in blocking), findings
PY
echo "check-vacuity.py flags check_vacuous and accepts check_midpoint"

set +e
"$SCRIPTS/check-assumptions.py" src > "$WORK/assumptions.json"
status=$?
set -e
[ "$status" -eq 2 ] || fail "check-assumptions.py should exit 2, got $status"
python3 - "$WORK/assumptions.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
assert r["harnesses_checked"] == 3, r
rules = {(f["harness"], f["rule"]) for f in r["findings"]}
assert ("check_vacuous", "contradictory-bounds") in rules, rules
assert not any(h == "check_midpoint" for h, _ in rules), rules
PY
echo "check-assumptions.py flags the contradictory assumptions"

"$SCRIPTS/inspect-harness.py" src --harness check_naive_midpoint > "$WORK/inspect.json"
python3 - "$WORK/inspect.json" <<'PY'
import json, sys
[h] = json.load(open(sys.argv[1]))["harnesses"]
assert h["counts"]["nondet_inputs"] == 2 and "naive_midpoint" in h["calls"], h
PY
echo "inspect-harness.py describes the harness"

# Concrete playback turns the counterexample into a small unit test.
cargo kani --harness check_naive_midpoint -Z concrete-playback --concrete-playback=print \
    > "$WORK/playback.log" 2>&1 || true
grep -q "Concrete playback unit test for \`proofs::check_naive_midpoint\`" "$WORK/playback.log" \
    || fail "no concrete playback test: $(tail -n 30 "$WORK/playback.log")"
grep -q "kani::concrete_playback_run" "$WORK/playback.log" || fail "malformed playback test"
echo "concrete playback produces a reproducer for the failing harness"

# AutoHarness has no MCP tool; it must work through the CLI, with JSON export. It also runs
# the manual harnesses, so the intentional failure makes it exit non-zero.
cargo kani autoharness -Z autoharness --include-pattern 'agent_plugin_fixture::midpoint' \
    -Z export-json --export-json "$WORK/auto.json" > "$WORK/auto.log" 2>&1 || true
[ -f "$WORK/auto.json" ] || fail "autoharness produced no export: $(tail -n 30 "$WORK/auto.log")"
python3 - "$WORK/auto.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
auto = [h for h in d["harnesses"] if h["is_automatically_generated"]]
assert auto and all(h["outcome"].get("verdict") == "SUCCESS" for h in auto), d["harnesses"]
PY
echo "autoharness runs through the CLI and exports JSON"
