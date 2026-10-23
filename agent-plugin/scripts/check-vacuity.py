#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Flag Kani proofs that may be vacuous.

A harness can report SUCCESS without proving anything: if its assumptions exclude every
input, or the code path containing an assertion is never reached, every check passes
trivially. Kani reports such assertions as UNREACHABLE, and `kani::cover!` probes the
assumptions cannot satisfy as UNSATISFIABLE. This script reads an `--export-json` document
and reports, for every successful harness:

- user assertions (`assert!`, `assert_eq!`, ...) that were never reached,
- covers that are unsatisfiable or unreachable (e.g. a `kani::cover!(true)` vacuity probe),
- harnesses where every check was unreachable,
- harnesses without any cover, as an `info` suggestion to add a vacuity probe.

Usage:
    cargo kani -Z export-json --export-json results.json ...
    check-vacuity.py results.json

Exit status: 0 when nothing is suspicious (only `info` findings), 2 when at least one
`warning` or `error` finding is reported, 1 on input errors.
"""

import argparse
import json
import sys

# Property classes that come from user-written code rather than Kani's default safety checks.
USER_ASSERTION_CLASSES = {"assertion", "assert", "kani::assert"}


def prop_class(prop_id):
    # Property IDs have the shape `<function>.<class>.<index>`.
    parts = prop_id.rsplit(".", 2)
    return parts[1] if len(parts) == 3 else ""


def check_harness(harness):
    findings = []
    name = harness["name"]
    checks = harness.get("checks", {})
    covers = harness.get("covers", {})

    unreachable_asserts = [
        pid for pid in checks.get("unreachable", []) if prop_class(pid) in USER_ASSERTION_CLASSES
    ]
    for pid in unreachable_asserts:
        findings.append({
            "harness": name,
            "severity": "error",
            "rule": "unreachable-assertion",
            "property": pid,
            "message": "assertion is never reached; it holds vacuously. Check the harness "
                       "assumptions and the path to this assertion.",
        })
    for kind in ("unsatisfiable", "unreachable"):
        for pid in covers.get(kind, []):
            findings.append({
                "harness": name,
                "severity": "warning",
                "rule": f"{kind}-cover",
                "property": pid,
                "message": f"cover is {kind.upper()}: the assumptions exclude this behavior. "
                "If it is a vacuity probe, the harness proves nothing.",
            })

    total = checks.get("total") or 0
    if total and len(checks.get("unreachable", [])) == total:
        findings.append({
            "harness": name,
            "severity": "error",
            "rule": "all-checks-unreachable",
            "message": "every check in this harness is unreachable; the harness body is "
                       "likely cut off by an assumption, an early return, or a panic.",
        })
    if not covers.get("total"):
        findings.append({
            "harness": name,
            "severity": "info",
            "rule": "no-vacuity-probe",
            "message": "no cover in this harness. To confirm the assumptions are satisfiable, "
                       "add `kani::cover!(true, \"end of harness reached\")` as its last "
                       "statement and re-run.",
        })
    return findings


def check(doc):
    findings = []
    skipped = []
    for harness in doc.get("harnesses", []):
        outcome = harness.get("outcome", {})
        if outcome.get("kind") != "COMPLETED" or outcome.get("verdict") != "SUCCESS":
            skipped.append(harness["name"])
            continue
        findings.extend(check_harness(harness))
    return {"findings": findings, "skipped_unsuccessful": skipped}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("export_json", help="file written by `--export-json`")
    args = parser.parse_args(argv)
    try:
        with open(args.export_json, encoding="utf-8") as f:
            doc = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"error: cannot read Kani JSON export `{args.export_json}`: {e}", file=sys.stderr)
        return 1
    result = check(doc)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 2 if any(f["severity"] != "info" for f in result["findings"]) else 0


if __name__ == "__main__":
    sys.exit(main())
