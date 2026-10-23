#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The checks that keep a proof honest: Kani-backed spec checks, the weakened-check report, the
# proof diff against a pinned baseline, defect and fix confirmation with a concrete playback
# witness, and evidence bundles.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"
cd "$FIXTURE"
git init -q . && git add -A && git -c user.name=t -c user.email=t@t commit -qm baseline

# Kani-backed spec checks on the untouched fixture, without modifying it.
set +e
"$SCRIPTS/check-specs.py" src --probe-harnesses > "$WORK/specs.json"
status=$?
set -e
[ "$status" -eq 2 ] || fail "check-specs.py should exit 2, got $status: $(cat "$WORK/specs.json")"
[ -z "$(git status --porcelain)" ] || fail "check-specs.py modified the crate"
python3 - "$WORK/specs.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
rules = {(f["function"], f["rule"]) for f in r["findings"]}
expected = {
    ("check_midpoint", "harness-end-reached"),
    ("check_naive_midpoint", "harness-end-reached"),
    ("check_vacuous", "harness-end-unreachable"),
    ("check_div_contract", "harness-end-reached"),
    ("checked_div", "requires-satisfiable"),
    ("checked_div", "requires-always-true"),
    ("impossible", "requires-unsatisfiable"),
}
assert rules == expected, (rules ^ expected, r)
PY
echo "check-specs.py: Kani finds the vacuous harness, the unsatisfiable and the trivial precondition"

# A run with a disabled check reports it.
cargo kani --harness check_naive_midpoint --no-overflow-checks \
    -Z export-json --export-json "$WORK/weak.json" > /dev/null 2>&1 || true
"$SCRIPTS/summarize-results.py" "$WORK/weak.json" > "$WORK/weak-summary.json"
python3 - "$WORK/weak-summary.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
assert s["weakened_checks"] == ["--no-overflow-checks"], s
PY
echo "summarize-results.py: reports --no-overflow-checks"

# Cheat: silence the overflow by changing the code, weaken a harness, strengthen a precondition.
python3 - <<'PY'
p = "src/lib.rs"
s = open(p).read()
s = s.replace("(a + b) / 2", "a.wrapping_add(b) / 2")
s = s.replace("""        let m = midpoint(a, b);
        assert!(a.min(b) <= m && m <= a.max(b));
        kani::cover!(true, "harness end reached");""", """        kani::assume(a < 100);
        let _ = midpoint(a, b);""")
s = s.replace("kani::requires(divisor != 0))]", "kani::requires(divisor != 0 && dividend > 5))]")
open(p, "w").write(s)
PY
set +e
"$SCRIPTS/check-proof-diff.py" src --base HEAD > "$WORK/diff.json"
status=$?
set -e
[ "$status" -eq 2 ] || fail "check-proof-diff.py should exit 2, got $status"
python3 - "$WORK/diff.json" <<'PY'
import json, sys
rules = {(f["item"], f["rule"]) for f in json.load(open(sys.argv[1]))["findings"]}
for expected in [("naive_midpoint", "code-changed"), ("check_midpoint", "assertion-removed"),
                 ("check_midpoint", "cover-removed"), ("check_midpoint", "assumption-added"),
                 ("checked_div", "requires-changed")]:
    assert expected in rules, (expected, rules)
assert not any(item == "midpoint" for item, _ in rules), rules
PY
echo "check-proof-diff.py: flags the code change, the weakened harness, and the new precondition"

set +e
"$SCRIPTS/check-specs.py" src --function checked_div --compare-base HEAD > "$WORK/cmp.json"
set -e
python3 - "$WORK/cmp.json" <<'PY'
import json, sys
rules = {f["rule"] for f in json.load(open(sys.argv[1]))["findings"]}
assert "requires-stronger" in rules, rules
PY
echo "check-specs.py --compare-base: Kani classifies the new precondition as stronger"
git checkout -q -- src

# Witness: confirm the defect, then confirm a fix against the baseline.
cargo kani --harness check_naive_midpoint -Z concrete-playback --concrete-playback=inplace \
    > "$WORK/inplace.log" 2>&1 || true
TEST="$(grep -o 'fn kani_concrete_playback_check_naive_midpoint_[0-9]*' src/lib.rs | head -n 1 | cut -c4-)"
[ -n "$TEST" ] || fail "no concrete playback test was added: $(tail -n 20 "$WORK/inplace.log")"
"$SCRIPTS/check-witness.py" . --test "$TEST" > "$WORK/witness.json" \
    || fail "defect not confirmed: $(cat "$WORK/witness.json")"
[ "$(jq_py "$WORK/witness.json" 'd["verdict"]')" = "defect-confirmed" ] || fail "expected defect-confirmed"
echo "check-witness.py: the witness confirms the defect"

sed -i.bak 's|(a + b) / 2|((a as u64 + b as u64) / 2) as u32|' src/lib.rs && rm -f src/lib.rs.bak
"$SCRIPTS/check-witness.py" . --test "$TEST" --base HEAD > "$WORK/fix.json" \
    || fail "fix not confirmed: $(cat "$WORK/fix.json")"
[ "$(jq_py "$WORK/fix.json" 'd["verdict"]')" = "fix-confirmed" ] || fail "expected fix-confirmed"
echo "check-witness.py: the witness fails before the fix and passes after it"
git checkout -q -- src

# Evidence: record a run, verify it by re-running, and detect a changed source.
CMD="cargo kani --harness check_midpoint -Z export-json --export-json $WORK/ev.json"
$CMD > /dev/null 2>&1 || fail "check_midpoint should pass"
"$SCRIPTS/evidence.py" record --export "$WORK/ev.json" --command "$CMD" --out "$WORK/evidence" > "$WORK/rec.json"
BUNDLE="$(jq_py "$WORK/rec.json" 'd["bundle"]')"
"$SCRIPTS/evidence.py" verify "$BUNDLE" > "$WORK/ver.json" || fail "verify failed: $(cat "$WORK/ver.json")"
echo "// edited" >> src/lib.rs
if "$SCRIPTS/evidence.py" verify "$BUNDLE" --no-rerun > "$WORK/ver2.json"; then
    fail "a changed source should be reported"
fi
grep -q "src/lib.rs changed since the run" "$WORK/ver2.json" || fail "unclear evidence difference"
echo "evidence.py: records, re-verifies, and detects a changed source"
