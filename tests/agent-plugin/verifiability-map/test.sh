#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The Crate Verifiability Map end to end: run Kani on a two-crate workspace, build the map with
# the results, plan bottom-up, merge new results, and detect sources that changed since the map.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"
MAP="$SCRIPTS/map-crate.py"
cp -R "$PLUGIN/tests/fixtures/workspace" "$WORK/ws"
cd "$WORK/ws"

# Real Kani results: four harnesses pass, check_fact fails.
cargo kani --workspace -Z function-contracts -Z export-json --export-json "$WORK/results.json" \
    > "$WORK/kani.log" 2>&1 || true
[ -f "$WORK/results.json" ] || fail "no export: $(tail -n 20 "$WORK/kani.log")"

CALLGRAPHS="text"
if command -v cargo-callgraph > /dev/null; then
    CALLGRAPHS="text tool"
fi
for CG in $CALLGRAPHS; do
    "$MAP" map . --callgraph "$CG" --results "$WORK/results.json" --out "$WORK/map-$CG.json" > /dev/null \
        || fail "map failed ($CG)"
    python3 - "$WORK/map-$CG.json" "$CG" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
by = {f["qualified_path"].split("::", 1)[1]: f for f in m["functions"]}
status = {k: v["verification"]["status"] for k, v in by.items()}
assert status["sum_to"] == "verified" and status["math::div"] == "verified", status
assert status["fact"] == "failed" and status["is_even"] == "not_attempted", status
assert by["run_twice"]["verification"]["status"] == "verified", by["run_twice"]["verification"]
assert by["fact"]["recursive"] and by["is_even"]["component"] == by["is_odd"]["component"], by["fact"]
assert "ws_core::sum_to" in by["run"]["callees"], by["run"]
assert by["shapes::total_area"]["unresolved_callees"], by["shapes::total_area"]
assert all(f["complexity"]["status"] == "not_computed" for f in m["functions"])
assert m["configuration"]["call_graph"] == ("cargo-callgraph" if sys.argv[2] == "tool" else "text-scan")
PY
done
echo "map: functions, call graph ($CALLGRAPHS), recursion, and Kani results merged"

# Deterministic output.
"$MAP" map . --callgraph text --results "$WORK/results.json" --out "$WORK/again.json" > /dev/null
cmp -s "$WORK/map-text.json" "$WORK/again.json" || fail "the map is not deterministic"
echo "map: deterministic output"

# Plan: bottom-up, verified functions skipped; a goal restricts the scope.
"$MAP" plan "$WORK/map-text.json" > "$WORK/plan.json"
"$MAP" plan "$WORK/map-text.json" --goal run_twice > "$WORK/plan-goal.json"
python3 - "$WORK/plan.json" "$WORK/plan-goal.json" <<'PY'
import json, sys
plan, goal = (json.load(open(p)) for p in sys.argv[1:])
todo = {i["function"].split("::", 1)[1]: i for s in plan["stages"] for i in s["functions"]}
assert "sum_to" not in todo and todo["fact"]["strategy"] == "triage-failure", todo.keys()
assert todo["is_even"]["strategy"] == "contract-and-recursion", todo["is_even"]
assert todo["shapes::total_area"]["unresolved_callees"], todo["shapes::total_area"]
assert [s["level"] for s in plan["stages"]] == sorted(s["level"] for s in plan["stages"])
assert goal["scope"]["to_do"] == 0, goal["scope"]  # run_twice and everything it uses are verified
PY
echo "plan: bottom-up stages, and a goal limits the scope"

# Update: a run that now also covers is_even; the map and plan follow.
python3 - "$WORK/results.json" "$WORK/results2.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["harnesses"] = [h for h in d["harnesses"] if h["name"] != "proofs::check_fact"]
d["started_at"] = "2099-01-01T00:00:00Z"
json.dump(d, open(sys.argv[2], "w"))
PY
cp "$WORK/map-text.json" "$WORK/map-updated.json"
"$MAP" update "$WORK/map-updated.json" --results "$WORK/results2.json" --root . > /dev/null \
    || fail "update failed"
python3 - "$WORK/map-updated.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
by = {f["qualified_path"].split("::", 1)[1]: f for f in m["functions"]}
assert by["fact"]["verification"]["status"] == "not_attempted", by["fact"]["verification"]
assert by["sum_to"]["verification"]["status"] == "verified"
PY
echo "update: statuses follow the new results"

# Sources that changed since the map was built make `update` refuse.
echo "// edit" >> core/src/math.rs
if "$MAP" update "$WORK/map-updated.json" --results "$WORK/results2.json" --root . > /dev/null 2> "$WORK/err.txt"; then
    fail "update should refuse when sources changed"
fi
grep -q "core/src/math.rs" "$WORK/err.txt" || fail "unclear message: $(cat "$WORK/err.txt")"
echo "update: refuses when the sources changed since the map was built"

"$MAP" explain "$WORK/map-text.json" fact > /dev/null || fail "explain failed"
"$MAP" report "$WORK/map-text.json" | grep -q "flowchart TD" || fail "report failed"
[ ! -e "$WORK/ws/Cargo.lock.orig" ] || fail "unexpected files"
smoke_pass
