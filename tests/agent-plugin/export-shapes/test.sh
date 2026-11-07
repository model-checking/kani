#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The plugin reads the export of the Kani under test, and gives the same answers as on the
# fixtures of the other export shape (Kani 0.68.0 writes the "tree" shape, current Kani the
# "flat" one): verdicts, failed properties, unreachable checks, unsatisfiable covers, and stubs.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"

cp -R "$PLUGIN/tests/fixtures/export-probe" "$WORK/probe"
cd "$WORK/probe"
FLAGS="$(PYTHONPATH="$SCRIPTS" python3 -c "
from kani_plugin_lib import export_args
print(' '.join(export_args('$WORK/run.json')))")"
echo "export flags for this Kani: $FLAGS"
# One failing harness makes Kani exit with status 1: that is the point of the probe crate.
# shellcheck disable=SC2086
kani src/lib.rs -Z function-contracts -Z stubbing $FLAGS > "$WORK/run.log" 2>&1 || true
[ -f "$WORK/run.json" ] || fail "Kani wrote no export: $(tail -5 "$WORK/run.log")"

"$SCRIPTS/summarize-results.py" "$WORK/run.json" --workspace "$WORK/probe" > "$WORK/summary.json"
for shape in flat tree; do
    "$SCRIPTS/summarize-results.py" "$PLUGIN/tests/fixtures/exports/probe-$shape.json" \
        --workspace "$WORK/probe" > "$WORK/summary-$shape.json"
done
python3 - "$WORK" <<'PY' || fail "the summary of this Kani's export differs from the fixtures"
import json, sys
work = sys.argv[1]
load = lambda n: json.load(open(f"{work}/{n}.json"))
now, flat, tree = load("summary"), load("summary-flat"), load("summary-tree")
keep = ("name", "verdict", "location", "failed", "unreachable_checks", "unsatisfiable_covers", "flags")
def view(s):
    out = {k: s[k] for k in ("total", "successful", "failed", "inconclusive", "all_ok")}
    out["harnesses"] = [{k: h[k] for k in keep} for h in s["harnesses"]]
    out["stubs"] = {h["name"]: h.get("trusted", {}).get("stubs") for h in s["harnesses"]}
    return out
for name, other in (("flat fixture", flat), ("tree fixture", tree)):
    if view(now) != view(other):
        print(f"differs from the {name}:\n{json.dumps(view(now), indent=1)}\n{json.dumps(view(other), indent=1)}")
        sys.exit(1)
assert now["total"] == 4 and now["failed"] == 1 and not now["all_ok"]
assert any(h["name"].endswith("stubbed") and h["trusted"]["stubs"] == ["expensive -> stub_expensive"]
           for h in now["harnesses"]), now
PY
echo "summarize-results.py gives the same answers on this Kani's export and on both fixtures"

"$SCRIPTS/check-vacuity.py" "$WORK/run.json" --workspace "$WORK/probe" > "$WORK/vacuity.json" || [ $? -eq 2 ] \
    || fail "check-vacuity.py failed: $(cat "$WORK/vacuity.json")"
python3 - "$WORK" <<'PY' || fail "check-vacuity.py findings differ"
import json, sys
rules = sorted({(f["harness"], f["rule"]) for f in json.load(open(sys.argv[1] + "/vacuity.json"))["findings"]})
assert ("proofs::passes", "unsatisfiable-cover") in rules, rules
assert ("proofs::passes", "unreachable-assertion") in rules, rules
PY
echo "check-vacuity.py reports the unreachable assertion and the unsatisfiable cover"

"$SCRIPTS/check-kani-version.py" --kani-cmd kani > "$WORK/version.json" \
    || fail "check-kani-version.py rejected this Kani: $(cat "$WORK/version.json")"
[ "$(python3 -c "import json;print(json.load(open('$WORK/version.json'))['export_readable'])")" = "True" ] \
    || fail "export_readable is not true: $(cat "$WORK/version.json")"
echo "check-kani-version.py reads this Kani's export"

smoke_pass
