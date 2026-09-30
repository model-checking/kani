#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check vacuous_pass and vacuity_suspect on the fixtures in test.rs, with and without reach checks.

set -eu
set -o pipefail

OUTPUT_FILE="vacuity_output.json"
NO_REACH_OUTPUT_FILE="vacuity_no_reach_output.json"
trap 'rm -f "$OUTPUT_FILE" "$NO_REACH_OUTPUT_FILE"' EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

code=0
kani -Z export-json test.rs --export-json "$OUTPUT_FILE" || code=$?
if [ "$code" -eq 0 ]; then
    echo "ERROR: expected nonzero exit status from the run with reach checks, got 0"
    exit 1
fi

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

code=0
kani -Z export-json -Z unstable-options --no-assertion-reach-checks test.rs \
    --export-json "$NO_REACH_OUTPUT_FILE" || code=$?
if [ "$code" -eq 0 ]; then
    echo "ERROR: expected nonzero exit status from the run without reach checks, got 0"
    exit 1
fi
if [ ! -f "$NO_REACH_OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $NO_REACH_OUTPUT_FILE was not created"
    exit 1
fi
python3 "$VALIDATOR" "$NO_REACH_OUTPUT_FILE" 2>&1 | tail -1

python3 << EOF
import json
import sys

sys.path.insert(0, "$PROJECT_ROOT/scripts")
from validate_json_export import vacuous_pass, vacuity_suspect

with open('$OUTPUT_FILE', 'r') as f:
    data = json.load(f)
with open('$NO_REACH_OUTPUT_FILE', 'r') as f:
    no_reach_data = json.load(f)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


EXPECTED = (
    'check_contradictory_assume', 'check_real_failure', 'check_real_success',
    'check_checkless_success', 'check_should_panic_success',
    'check_should_panic_failing_vacuous',
)
by_name = {h['name'].rsplit('::', 1)[-1]: h for h in data['harnesses']}
check(set(by_name) == set(EXPECTED),
      f"exact harness set mismatch in the reach-checks-on run: got {sorted(by_name)}, "
      f"expected {sorted(EXPECTED)}")

h = by_name.get('check_contradictory_assume')
if h is not None:
    checks = h['checks']
    check(h['outcome'].get('verdict') == 'SUCCESS',
          f"outcome.verdict should be SUCCESS, got {h['outcome'].get('verdict')}")
    check(checks['total'] == 2, f"checks.total should be 2, got {checks['total']}")
    check(len(checks['unreachable']) == checks['total'],
          f"checks.unreachable.len() ({len(checks['unreachable'])}) should equal checks.total "
          f"({checks['total']}) -- this is the RFC 0015 normative vacuous-pass predicate")
    check(checks['success'] == 0,
          f"checks.success should be 0 (every check is unreachable, not successful), got "
          f"{checks['success']}")
    check(data['configuration']['checks']['assertion_reach_checks'] is True,
          "assertion_reach_checks must be true for the vacuous-pass predicate to be sound")
    check(h['n_failed'] == 0, f"n_failed should be 0, got {h['n_failed']}")
    check(vacuous_pass(h, data['configuration']) is True,
          "vacuous_pass(check_contradictory_assume) should be True")

h = by_name.get('check_real_failure')
if h is not None:
    check(h['outcome'].get('verdict') == 'FAILURE',
          f"check_real_failure: outcome.verdict should be FAILURE, got {h['outcome'].get('verdict')}")
    check(vacuous_pass(h, data['configuration']) is False,
          "vacuous_pass(check_real_failure) should be False (not a SUCCESS verdict at all)")

h = by_name.get('check_real_success')
if h is not None:
    checks = h['checks']
    check(h['outcome'].get('verdict') == 'SUCCESS',
          f"check_real_success: outcome.verdict should be SUCCESS, got {h['outcome'].get('verdict')}")
    check(checks['total'] > 0, f"check_real_success: checks.total should be > 0, got {checks['total']}")
    check(len(checks['unreachable']) == 0,
          f"check_real_success: checks.unreachable should be empty, got {checks['unreachable']}")
    check(vacuous_pass(h, data['configuration']) is False,
          "vacuous_pass(check_real_success) should be False (a real, non-vacuous SUCCESS)")

h = by_name.get('check_checkless_success')
if h is not None:
    checks = h['checks']
    check(h['outcome'].get('verdict') == 'SUCCESS',
          f"check_checkless_success: outcome.verdict should be SUCCESS, got {h['outcome'].get('verdict')}")
    check(checks['total'] == 0,
          f"check_checkless_success: checks.total should be 0, got {checks['total']}")
    check(vacuous_pass(h, data['configuration']) is False,
          "vacuous_pass(check_checkless_success) should be False: checks.total > 0 excludes a "
          "checkless harness even though its verdict is SUCCESS")

h = by_name.get('check_should_panic_success')
if h is not None:
    check(h['outcome'].get('verdict') == 'SUCCESS',
          f"check_should_panic_success: outcome.verdict should be SUCCESS, got "
          f"{h['outcome'].get('verdict')}")
    check(h['failure_kind'] == 'PANICS_ONLY',
          f"check_should_panic_success: failure_kind should be PANICS_ONLY, got {h['failure_kind']}")
    check(len(h['failed_properties']) > 0,
          "check_should_panic_success: failed_properties should be non-empty (the panic "
          "assertion itself is FAILURE, even though the harness verdict is SUCCESS)")
    check(vacuous_pass(h, data['configuration']) is False,
          "vacuous_pass(check_should_panic_success) should be False: it has real, non-unreachable "
          "checks despite the expected-panic SUCCESS verdict")

h = by_name.get('check_should_panic_failing_vacuous')
if h is not None:
    checks = h['checks']
    check(h['outcome'].get('verdict') == 'FAILURE',
          f"check_should_panic_failing_vacuous: outcome.verdict should be FAILURE (should_panic "
          f"expected a panic that never happened), got {h['outcome'].get('verdict')}")
    check(h['failure_kind'] == 'NONE',
          f"check_should_panic_failing_vacuous: failure_kind should be NONE (no property ever "
          f"fails; the checks are all unreachable), got {h['failure_kind']}")
    check(checks['total'] == 2, f"check_should_panic_failing_vacuous: checks.total should be 2, "
          f"got {checks['total']}")
    check(len(checks['unreachable']) == checks['total'],
          "check_should_panic_failing_vacuous: checks.unreachable.len() should equal checks.total "
          "-- same shape as check_contradictory_assume, but with a non-SUCCESS verdict")
    check(vacuous_pass(h, data['configuration']) is False,
          "vacuous_pass(check_should_panic_failing_vacuous) should be False: verdict != SUCCESS, "
          "even though every check is unreachable -- catches a missing verdict guard")
    check(vacuity_suspect(h, data['configuration']) is False,
          "vacuity_suspect(check_should_panic_failing_vacuous) should be False: verdict != SUCCESS "
          "gates this predicate too")

no_reach_by_name = {h['name'].rsplit('::', 1)[-1]: h for h in no_reach_data['harnesses']}
check(set(no_reach_by_name) == set(EXPECTED),
      f"exact harness set mismatch in the reach-checks-off run: got {sorted(no_reach_by_name)}, "
      f"expected {sorted(EXPECTED)}")
check(no_reach_data['configuration']['checks']['assertion_reach_checks'] is False,
      "assertion_reach_checks should be false in the --no-assertion-reach-checks run")
for name, h in no_reach_by_name.items():
    check(vacuous_pass(h, no_reach_data['configuration']) is None,
          f"vacuous_pass({name}) should be None (not applicable) with assertion_reach_checks off")
    check(vacuity_suspect(h, no_reach_data['configuration']) is None,
          f"vacuity_suspect({name}) should be None (not applicable) with assertion_reach_checks off")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("vacuous_pass: True only for the contradictory-assume harness, False for every real "
      "SUCCESS/FAILURE negative fixture, None (not applicable) with assertion_reach_checks off")
EOF
