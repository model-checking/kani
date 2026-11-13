#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The full skill workflow without the MCP server: run Kani through the CLI with JSON export,
# summarize results, produce a concrete playback reproducer, check for vacuity and suspicious
# assumptions, and run AutoHarness (which has no MCP tool).

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"
cd "$FIXTURE"

# Verify all harnesses. One harness fails, so Kani exits non-zero.
if cargo kani -Z function-contracts -Z export-json --export-json "$WORK/all.json" > "$WORK/all.log" 2>&1; then
    fail "expected check_naive_midpoint to fail"
fi
[ -f "$WORK/all.json" ] || fail "no JSON export: $(tail -n 30 "$WORK/all.log")"

"$SCRIPTS/summarize-results.py" "$WORK/all.json" > "$WORK/summary.json"
python3 - "$WORK/summary.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
by_name = {h["name"].split("::")[-1]: h for h in s["harnesses"]}
assert s["total"] == 5 and s["failed"] == 1, s
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
"$SCRIPTS/check-vacuity.py" "$WORK/all.json" --log "$WORK/all.log" > "$WORK/vacuity.json"
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
python3 - "$WORK/vacuity.json" <<'PY'
import json, sys
findings = json.load(open(sys.argv[1]))["findings"]
# With the log, Kani's built-in checks are never reported as unreachable user assertions.
for f in findings:
    if f["rule"] == "unreachable-assertion":
        assert not f["description"].strip('"').startswith("attempt to"), f
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
assert r["harnesses_checked"] == 5, r
rules = {(f["harness"], f["rule"]) for f in r["findings"]}
assert ("check_vacuous", "contradictory-bounds") in rules, rules
assert not any(h == "check_midpoint" for h, _ in rules), rules
PY
echo "check-assumptions.py flags the contradictory assumptions"

"$SCRIPTS/inspect-harness.py" src --summary > "$WORK/inspect-summary.json"
[ "$(jq_py "$WORK/inspect-summary.json" 'd["harnesses"]')" = "5" ] || fail "inspect-harness.py --summary"
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

# AutoHarness must work through the CLI too, with JSON export. It also runs the manual
# harnesses, so the intentional failure makes it exit non-zero.
cargo kani autoharness -Z autoharness -Z unstable-options --harness-timeout 600s --include-pattern 'agent_plugin_fixture::midpoint' \
    -Z export-json --export-json "$WORK/auto.json" > "$WORK/auto.log" 2>&1 || true
[ -f "$WORK/auto.json" ] || fail "autoharness produced no export: $(tail -n 30 "$WORK/auto.log")"
python3 - "$WORK/auto.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
auto = [h for h in d["harnesses"] if h["is_automatically_generated"]]
assert auto and all(h["outcome"].get("verdict") == "SUCCESS" for h in auto), d["harnesses"]
PY
echo "autoharness runs through the CLI and exports JSON"

# Coverage: the vacuous harness never reaches `midpoint`, so no harness covers it.
cargo kani -Z function-contracts -Z source-coverage --coverage --harness check_vacuous \
    > "$WORK/cov.log" 2>&1 || fail "coverage run failed: $(tail -n 20 "$WORK/cov.log")"
"$SCRIPTS/summarize-coverage.py" --log "$WORK/cov.log" --file src/lib.rs > "$WORK/cov.json" \
    || fail "summarize-coverage.py failed"
python3 - "$WORK/cov.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
[f] = d["files"]
assert f["file"] == "src/lib.rs" and f["percent"] < 100, f
assert "midpoint" in {r["function"] for r in f["uncovered"]}, f
PY
cargo kani -Z function-contracts -Z source-coverage --coverage --harness check_midpoint \
    > "$WORK/cov2.log" 2>&1 || fail "coverage run failed: $(tail -n 20 "$WORK/cov2.log")"
"$SCRIPTS/summarize-coverage.py" --log "$WORK/cov2.log" --file src/lib.rs > "$WORK/cov2.json"
python3 - "$WORK/cov2.json" <<'PY'
import json, sys
[f] = json.load(open(sys.argv[1]))["files"]
assert "midpoint" not in {r["function"] for r in f.get("uncovered", [])}, f
PY
echo "summarize-coverage.py: finds the code no harness exercises, and sees it covered once a harness reaches it"

# list-harnesses.py: a clean listing, no clutter in the project, features detected from the sources.
"$SCRIPTS/list-harnesses.py" . > "$WORK/list.json" || fail "list-harnesses.py failed: $(cat "$WORK/list.json")"
[ ! -e kani-list.json ] || fail "list-harnesses.py left kani-list.json behind"
[ "$(jq_py "$WORK/list.json" 'd["totals"]["contract-harnesses"]')" = "2" ] || fail "unexpected listing: $(cat "$WORK/list.json")"
echo "list-harnesses.py: lists without leaving files behind"

# run-harnesses.py: one process per harness, merged export kept as results arrive.
"$SCRIPTS/run-harnesses.py" . --harness check_midpoint --harness check_vacuous --harness check_naive_midpoint \
    --timeout-per 300 --out "$WORK/runs" > "$WORK/runs.json" 2> "$WORK/runs.err" || true
python3 - "$WORK/runs/export.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
v = {h["name"].split("::")[-1]: (h["outcome"].get("verdict") or h["outcome"]["kind"]) for h in d["harnesses"]}
assert v == {"check_midpoint": "SUCCESS", "check_vacuous": "SUCCESS", "check_naive_midpoint": "FAILURE"}, v
assert d["run_state"] == "COMPLETE" and d["summary"]["total"] == 3, d["summary"]
PY
"$SCRIPTS/summarize-results.py" "$WORK/runs/export.json" > "$WORK/runs-summary.json"
python3 - "$WORK/runs-summary.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
assert (s["total"], s["successful"], s["failed"], s["inconclusive"], s["all_ok"]) == (3, 2, 1, 0, False), s
assert all("unreachable_checks" in h and "flags" in h for h in s["harnesses"]), s["harnesses"][0]
PY
echo "run-harnesses.py: runs each harness in its own process and merges the results"

# A harness filter that matches nothing is an error, and a module-qualified name works.
if "$SCRIPTS/check-assumptions.py" src --harness no_such_harness > /dev/null 2> "$WORK/err.txt"; then
    fail "an unmatched harness filter should fail"
fi
grep -q "no harness matches" "$WORK/err.txt" || fail "unclear error: $(cat "$WORK/err.txt")"
"$SCRIPTS/check-assumptions.py" src --harness proofs::check_vacuous > "$WORK/qual.json" || [ $? -eq 2 ]
[ "$(jq_py "$WORK/qual.json" 'd["harnesses_checked"]')" = "1" ] || fail "module-qualified filter did not match"
echo "harness filters: qualified names work, and a filter that matches nothing is an error"

# check-vacuity.py counts info findings instead of listing them.
python3 - "$WORK/vacuity.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert "info_summary" in d and all(f["severity"] != "info" for f in d["findings"]), d.keys()
assert all("description" in f for f in d["findings"]), d["findings"]
PY
echo "check-vacuity.py: info findings are counted, every finding has a description"

# cleanup-playback.py removes duplicated generated tests.
cat > "$WORK/dup.rs" <<'RS'
fn h() {}

/// Test generated for harness `h`
///
/// Check for `assertion`: "x"

#[test]
fn kani_concrete_playback_h_1() {
    kani::concrete_playback_run(vec![], h);
}

/// Test generated for harness `h`
///
/// Check for `assertion`: "x"

#[test]
fn kani_concrete_playback_h_1() {
    kani::concrete_playback_run(vec![], h);
}
RS
"$SCRIPTS/cleanup-playback.py" "$WORK/dup.rs" > /dev/null
[ "$(grep -c 'fn kani_concrete_playback_h_1' "$WORK/dup.rs")" = "1" ] || fail "duplicate test not removed"
echo "cleanup-playback.py: removes duplicated generated tests"
smoke_pass
