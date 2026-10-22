#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""
Checks scripts/validate_json_export.py against the fixtures in `fixture-*.json`.

Every fixture must validate. Each case below breaks one rule in a copy of a valid fixture;
the validator must reject it with a diagnostic naming the broken field, and must not crash.
Each extension below adds a field RFC 0015 does not define to a copy; the validator must accept it.

Usage: check_fixtures.py <path to validate_json_export.py>
"""

import copy
import json
import subprocess
import sys
from pathlib import Path

FIXTURES = Path(__file__).parent
COMPLETED = "fixture-completed-failure"
NON_COMPLETED = "fixture-non-completed-outcomes"


def parts(path):
    return [int(p) if p.isdigit() else p for p in path.split(".")]


def set_at(path, value):
    def mutate(doc):
        *parents, last = parts(path)
        target = doc
        for key in parents:
            target = target[key]
        target[last] = value
    return mutate


def delete_at(path):
    def mutate(doc):
        *parents, last = parts(path)
        target = doc
        for key in parents:
            target = target[key]
        del target[last]
    return mutate


def all_of(*mutations):
    def mutate(doc):
        for mutation in mutations:
            mutation(doc)
    return mutate


H0 = "harnesses.0"
# (case name, fixture, mutation, text the diagnostic must contain)
CASES = [
    # Scalar leaf types.
    ("integer warning message", COMPLETED, set_at(f"{H0}.warnings.0.message", 7),
     "harnesses[0].warnings[0].message"),
    ("integer property id", COMPLETED, set_at(f"{H0}.failed_properties.0.id", 7),
     "harnesses[0].failed_properties[0].id"),
    ("integer line in a property", COMPLETED, set_at(f"{H0}.failed_properties.0.line", 10),
     "harnesses[0].failed_properties[0].line"),
    ("string trace_available", COMPLETED, set_at(f"{H0}.failed_properties.0.trace_available", "yes"),
     "harnesses[0].failed_properties[0].trace_available"),
    ("boolean harness line", COMPLETED, set_at(f"{H0}.line", True), "harnesses[0].line"),
    ("string wall_time_s", COMPLETED, set_at("wall_time_s", "fast"), "wall_time_s"),
    ("malformed started_at", COMPLETED, set_at("started_at", "yesterday"), "started_at"),
    ("lowercase status", COMPLETED, set_at(f"{H0}.failed_properties.0.status", "Failure"),
     "harnesses[0].failed_properties[0].status"),
    ("unknown failure_kind", COMPLETED, set_at(f"{H0}.failure_kind", "BROKEN"),
     "harnesses[0].failure_kind"),
    ("unknown outcome kind", NON_COMPLETED, set_at(f"{H0}.outcome.kind", "ABORTED"),
     "harnesses[0].outcome.kind"),
    ("unsupported schema version", COMPLETED, set_at("schema_version", "9.9.9"), "schema_version"),
    ("null where not allowed", COMPLETED, set_at("tools.kani", None), "tools.kani"),
    ("null covers.satisfied", COMPLETED, set_at(f"{H0}.covers.satisfied", None),
     "harnesses[0].covers.satisfied"),
    ("null checks.unreachable", COMPLETED, set_at(f"{H0}.checks.unreachable", None),
     "harnesses[0].checks.unreachable"),
    ("scalar checks.other", COMPLETED, set_at(f"{H0}.checks.other", 9), "harnesses[0].checks.other"),
    ("solver of the wrong shape", COMPLETED, set_at(f"{H0}.attributes.solver", {"Path": "x"}),
     "harnesses[0].attributes.solver"),
    ("attribute kind of the wrong shape", COMPLETED, set_at(f"{H0}.attributes.kind", "Kani"),
     "harnesses[0].attributes.kind"),
    ("missing required field", COMPLETED, delete_at(f"{H0}.resources.verification_time_s"),
     "harnesses[0].resources.verification_time_s"),
    # Elements of arrays that are empty in some fixtures.
    ("integer in cbmc_args", COMPLETED, set_at("configuration.cbmc_args", [5]),
     "configuration.cbmc_args[0]"),
    ("integer in requested_filters", COMPLETED, set_at("harness_selection.requested_filters", [1]),
     "harness_selection.requested_filters[0]"),
    ("integer in enabled_unstable_features", COMPLETED, set_at("enabled_unstable_features", [3]),
     "enabled_unstable_features[0]"),
    ("malformed stub", COMPLETED, set_at(f"{H0}.attributes.stubs", [{"original": 1, "replacement": "f"}]),
     "harnesses[0].attributes.stubs[0].original"),
    ("integer verified stub", COMPLETED, set_at(f"{H0}.attributes.verified_stubs", [1]),
     "harnesses[0].attributes.verified_stubs[0]"),
    ("integer in checks.failure", COMPLETED, set_at(f"{H0}.checks.failure", [4]),
     "harnesses[0].checks.failure[0]"),
    ("malformed checks.other entry", COMPLETED, set_at(f"{H0}.checks.other", [{"id": 5, "status": "SUCCESS"}]),
     "harnesses[0].checks.other[0].id"),
    ("malformed covers.other entry", COMPLETED, set_at(f"{H0}.covers.other", [{"id": "c"}]),
     "harnesses[0].covers.other[0].status"),
    ("incomplete unsupported construct", COMPLETED, set_at(f"{H0}.unsupported_constructs", [{}]),
     "harnesses[0].unsupported_constructs[0].id"),
    ("malformed warning", COMPLETED, set_at(f"{H0}.warnings", [{"message": "m"}]),
     "harnesses[0].warnings[0].truncated"),
    ("integer harness in an otherwise valid list", NON_COMPLETED, set_at("harnesses.2", 7),
     "harnesses[2]"),
    # Presence by outcome.
    ("COMPLETED without a verdict", COMPLETED, delete_at(f"{H0}.outcome.verdict"),
     "outcome.verdict must be present exactly on COMPLETED"),
    ("COMPLETED without failure_kind", COMPLETED, delete_at(f"{H0}.failure_kind"),
     "failure_kind must be present exactly on COMPLETED"),
    ("COMPLETED with a null n_properties", COMPLETED, set_at(f"{H0}.n_properties", None),
     "n_properties must be an integer on COMPLETED"),
    ("COMPLETED with a null checks.total", COMPLETED, set_at(f"{H0}.checks.total", None),
     "checks.total must be an integer on COMPLETED"),
    ("COMPLETED with a CRASHED code", COMPLETED, set_at(f"{H0}.outcome.code", 1),
     "outcome.code must be present exactly on CRASHED"),
    ("verdict contradicting failure_kind", COMPLETED, set_at(f"{H0}.outcome.verdict", "SUCCESS"),
     "does not follow from"),
    ("TIMEOUT carrying a verdict", NON_COMPLETED, set_at(f"{H0}.outcome.verdict", "SUCCESS"),
     "outcome.verdict must be present exactly on COMPLETED"),
    ("TIMEOUT carrying failure_kind", NON_COMPLETED, set_at(f"{H0}.failure_kind", "NONE"),
     "failure_kind must be present exactly on COMPLETED"),
    ("TIMEOUT with an integer checks.total", NON_COMPLETED, set_at(f"{H0}.checks.total", 0),
     "checks.total must be null on TIMEOUT"),
    ("TIMEOUT with an integer n_failed", NON_COMPLETED, set_at(f"{H0}.n_failed", 0),
     "n_failed must be null on TIMEOUT"),
    ("TIMEOUT with a non-empty checks.failure", NON_COMPLETED, set_at(f"{H0}.checks.failure", ["a.b.1"]),
     "checks.failure must be [] on TIMEOUT"),
    ("OUT_OF_MEMORY with a satisfied cover", NON_COMPLETED, set_at("harnesses.1.covers.satisfied", ["c"]),
     "covers.satisfied must be [] on OUT_OF_MEMORY"),
    ("OUT_OF_MEMORY with a failed property", NON_COMPLETED,
     set_at("harnesses.1.failed_properties", [{"id": "a"}]),
     "failed_properties"),
    ("CRASHED without a code", NON_COMPLETED, delete_at("harnesses.2.outcome.code"),
     "outcome.code must be present exactly on CRASHED"),
    ("CRASHED without a message", NON_COMPLETED, delete_at("harnesses.2.outcome.message"),
     "outcome.message must be present exactly on CRASHED"),
    ("CRASHED with a verdict", NON_COMPLETED, set_at("harnesses.2.outcome.verdict", "FAILURE"),
     "outcome.verdict must be present exactly on COMPLETED"),
    ("run-level outcome with a verdict", COMPLETED, set_at("outcome", {"kind": "COMPLETED", "verdict": "SUCCESS"}),
     "run-level outcome"),
    ("run-level outcome that is not COMPLETED", NON_COMPLETED, set_at("outcome.kind", "TIMEOUT"),
     "outcome.kind"),
]

# (case name, fixture, mutation): unknown fields that consumers must ignore.
EXTENSIONS = [
    ("unknown top-level field", COMPLETED, set_at("future_field", {"a": 1})),
    ("unknown run-level outcome member", COMPLETED, set_at("outcome.extra", True)),
    ("unknown harness outcome member", COMPLETED, set_at(f"{H0}.outcome.extra", True)),
    ("unknown member of a COMPLETED checks object", COMPLETED, set_at(f"{H0}.checks.extra", ["future"])),
    ("unknown member of a TIMEOUT checks object", NON_COMPLETED, set_at(f"{H0}.checks.extra", ["future"])),
    ("unknown member of an OUT_OF_MEMORY covers object", NON_COMPLETED,
     set_at("harnesses.1.covers.extra", ["future"])),
    ("unknown field beside a Binary solver", COMPLETED,
     set_at(f"{H0}.attributes.solver", {"Binary": "my-solver", "extra": 1})),
    ("unknown field beside a ProofForContract", COMPLETED,
     set_at(f"{H0}.attributes.kind", {"ProofForContract": {"target_fn": "f"}, "extra": 1})),
    ("unknown field inside a ProofForContract", COMPLETED,
     set_at(f"{H0}.attributes.kind", {"ProofForContract": {"target_fn": "f", "extra": 1}})),
]


def run_validator(validator, path):
    result = subprocess.run(
        [sys.executable, validator, str(path)], capture_output=True, text=True
    )
    return result.returncode, result.stdout + result.stderr


def main():
    validator = sys.argv[1]
    failures = []

    for fixture in sorted(FIXTURES.glob("fixture-*.json")):
        code, output = run_validator(validator, fixture)
        if code != 0:
            failures.append(f"valid fixture {fixture.name} was rejected:\n{output}")
        else:
            print(f"  {fixture.name}: accepted")

    scratch = Path("fixture_under_test.json")
    try:
        for name, fixture, mutate, expected in CASES:
            with open(FIXTURES / f"{fixture}.json") as f:
                doc = copy.deepcopy(json.load(f))
            mutate(doc)
            scratch.write_text(json.dumps(doc))
            code, output = run_validator(validator, scratch)
            if code == 0:
                failures.append(f"{name}: the validator accepted a malformed export")
            elif "Traceback" in output:
                failures.append(f"{name}: the validator crashed:\n{output}")
            elif expected not in output:
                failures.append(f"{name}: diagnostic does not mention {expected!r}:\n{output}")
            else:
                print(f"  {name}: correctly rejected")
        for name, fixture, mutate in EXTENSIONS:
            with open(FIXTURES / f"{fixture}.json") as f:
                doc = copy.deepcopy(json.load(f))
            mutate(doc)
            scratch.write_text(json.dumps(doc))
            code, output = run_validator(validator, scratch)
            if code != 0:
                failures.append(f"{name}: the validator rejected an unknown field:\n{output}")
            else:
                print(f"  {name}: accepted")
    finally:
        scratch.unlink(missing_ok=True)

    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
