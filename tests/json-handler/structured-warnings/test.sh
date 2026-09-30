#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that warnings[] caps and retention hold on every outcome, and that CRASHED, TIMEOUT and
# OUT_OF_MEMORY harnesses are exported with the right fields and times. The non-completed
# outcomes come from a fake cbmc.

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

REAL_CBMC="$(command -v cbmc)"

ARTIFACTS=(cap_under.json quantifier.json completed.json crashed.json timeout.json oom.json)
FAKE_CBMC_DIR=""
cleanup() {
    rm -f "${ARTIFACTS[@]}"
    if [ -n "$FAKE_CBMC_DIR" ]; then
        rm -rf "$FAKE_CBMC_DIR"
    fi
}
trap cleanup EXIT

FAKE_CBMC_DIR="$(mktemp -d)"
cat > "$FAKE_CBMC_DIR/cbmc" << EOF
#!/usr/bin/env bash
export REAL_CBMC="$REAL_CBMC"
exec python3 "$SCRIPT_DIR/fake_cbmc.py" "\$@"
EOF
chmod +x "$FAKE_CBMC_DIR/cbmc"

run_case() {
    local mode="$1" output="$2" want_exit="$3"
    shift 3
    local code=0
    FAKE_WARNING_MODE="$mode" PATH="$FAKE_CBMC_DIR:$PATH" \
        kani -Z export-json "$@" "$SCRIPT_DIR/fixture.rs" --export-json "$output" || code=$?
    if [ "$want_exit" = zero ] && [ "$code" -ne 0 ]; then
        echo "ERROR: mode=$mode exited $code, expected 0"
        exit 1
    fi
    if [ "$want_exit" = nonzero ] && [ "$code" -eq 0 ]; then
        echo "ERROR: mode=$mode exited 0, expected a nonzero status"
        exit 1
    fi
    if [ ! -f "$output" ]; then
        echo "ERROR: JSON file $output was not created (mode=$mode)"
        exit 1
    fi
    python3 "$VALIDATOR" "$output" 2>&1 | tail -1
}

run_case cap_under cap_under.json zero
run_case quantifier quantifier.json nonzero
run_case completed completed.json zero
run_case crashed crashed.json nonzero
run_case timeout timeout.json nonzero -Z unstable-options --harness-timeout 2s
run_case oom oom.json nonzero

SCRIPT_DIR="$SCRIPT_DIR" python3 << 'PYEOF'
import json
import os
import subprocess
import sys

MAX_WARNINGS = 20
MAX_MESSAGE_CHARS = 4096
# The fake cbmc sleeps 1 s before it crashes or is killed; a timeout run is cut off at 2 s. A
# verification time in milliseconds or in a larger unit than seconds falls outside this window.
MIN_TIME_S = {"CRASHED": 1.0, "OUT_OF_MEMORY": 1.0, "TIMEOUT": 2.0}
MAX_TIME_S = 60.0

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


def raw_warnings(mode):
    fake = os.path.join(os.environ["SCRIPT_DIR"], "fake_cbmc.py")
    out = subprocess.run([sys.executable, fake, "--print-warnings", mode],
                         capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def expected_records(raw):
    records = []
    for message in raw[:MAX_WARNINGS]:
        over = len(message) > MAX_MESSAGE_CHARS
        records.append({"message": message[:MAX_MESSAGE_CHARS], "truncated": over,
                        "original_chars": len(message) if over else None})
    return records, max(0, len(raw) - MAX_WARNINGS)


def first_difference(actual, expected):
    if len(actual) != len(expected):
        return f"{len(actual)} records, expected {len(expected)}"
    for i, (a, e) in enumerate(zip(actual, expected)):
        if a == e:
            continue
        for key in e:
            if a.get(key) == e[key]:
                continue
            if key == "message" and isinstance(a.get(key), str):
                at = next((j for j, (x, y) in enumerate(zip(a[key], e[key])) if x != y),
                          min(len(a[key]), len(e[key])))
                return (f"record {i} message differs at char {at} "
                        f"(got {len(a[key])} chars, expected {len(e[key])})")
            return f"record {i} {key}: got {a.get(key)!r}, expected {e[key]!r}"
        return f"record {i} has keys {sorted(a)}, expected {sorted(e)}"
    return "no difference"


def check_non_completed(path, h, kind):
    outcome = h["outcome"]
    check("verdict" not in outcome, f"{path}: outcome.verdict present on {kind} (must be absent)")
    check("failure_kind" not in h, f"{path}: failure_kind present on {kind} (must be absent)")
    for field in ("n_properties", "n_failed"):
        check(h[field] is None, f"{path}: {field} should be null on {kind}, got {h[field]}")
    for group, field in (("checks", "total"), ("checks", "success"), ("covers", "total")):
        check(h[group][field] is None,
              f"{path}: {group}.{field} should be null on {kind}, got {h[group][field]}")
    if kind == "CRASHED":
        check(outcome.get("code") == 1,
              f"{path}: outcome.code should be 1 (the fake cbmc's exit status), got "
              f"{outcome.get('code')!r}")
        check("message" in outcome and isinstance(outcome["message"], (str, type(None))),
              f"{path}: outcome.message should be present as a string or null, got "
              f"{outcome.get('message', '<absent>')!r}")
    else:
        check("code" not in outcome and "message" not in outcome,
              f"{path}: outcome.code/message present on {kind} (must be absent)")
    seconds = h["resources"]["verification_time_s"]
    check(MIN_TIME_S[kind] <= seconds < MAX_TIME_S,
          f"{path}: verification_time_s {seconds} not in [{MIN_TIME_S[kind]}, {MAX_TIME_S}) -- "
          "expected the elapsed seconds until the outcome")


def check_case(path, mode, kind, verdict=None):
    with open(path) as f:
        doc = json.load(f)
    if len(doc["harnesses"]) != 1:
        failures.append(f"{path}: expected 1 harness, got {len(doc['harnesses'])}")
        return None
    h = doc["harnesses"][0]
    check(h["outcome"]["kind"] == kind,
          f"{path}: expected outcome.kind {kind}, got {h['outcome']['kind']}")
    if verdict is not None:
        check(h["outcome"].get("verdict") == verdict,
              f"{path}: expected outcome.verdict {verdict}, got {h['outcome'].get('verdict')}")
    if kind != "COMPLETED":
        check_non_completed(path, h, kind)
    records, omitted = expected_records(raw_warnings(mode))
    check(h["warnings"] == records,
          f"{path}: warnings[] differs from the expected records: "
          f"{first_difference(h['warnings'], records)}")
    check(h["warnings_truncated"] == omitted,
          f"{path}: warnings_truncated should be {omitted}, got {h['warnings_truncated']}")
    return h


stress = raw_warnings("completed")
check(len(stress) == MAX_WARNINGS + 2,
      f"stress set should exceed the count cap by 2, got {len(stress)}")
check(sorted({len(m) for m in stress if len(m) >= MAX_MESSAGE_CHARS}) == [4096, 4097],
      "stress set should hold one message exactly at the char cap and one just over it")
check(any(len(m.encode()) > MAX_MESSAGE_CHARS + 1 for m in stress),
      "stress set should have multibyte messages, so chars and bytes differ")
for mode in ("crashed", "timeout", "oom"):
    check(raw_warnings(mode) == stress, f"{mode} must use the same stress set as completed")

records, omitted = expected_records(stress)
check(omitted == 2 and [r["truncated"] for r in records].count(True) == 1,
      "the stress set should drop 2 warnings and truncate exactly 1 message")

check_case("cap_under.json", "cap_under", "COMPLETED", "SUCCESS")
check_case("completed.json", "completed", "COMPLETED", "SUCCESS")
check_case("crashed.json", "crashed", "CRASHED")
check_case("timeout.json", "timeout", "TIMEOUT")
check_case("oom.json", "oom", "OUT_OF_MEMORY")

h = check_case("quantifier.json", "quantifier", "COMPLETED", "FAILURE")
if h is not None:
    check(h["failure_kind"] == "ERROR",
          f"quantifier.json: expected failure_kind ERROR, got {h['failure_kind']}")
    check(h["checks"]["error"] == [],
          f"quantifier.json: checks.error should be empty, got {h['checks']['error']}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("structured-warnings: both caps, exact record text, omitted counts, the ignored-quantifier "
      "ERROR classification and the CRASHED/TIMEOUT/OUT_OF_MEMORY field shapes and elapsed times "
      "match RFC 0015")
PYEOF

echo "All structured-warnings checks passed!"
