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

The export lists unreachable checks by ID only, and Kani's built-in checks (arithmetic
overflow, division by zero, index bounds) share the `assertion` class with user assertions.
Without more information an unreachable `assertion` is therefore a `warning`: often it is a
built-in check the solver proved cannot fail on any path. Pass Kani's console output with
`--log` (regular output format, the default) to classify each one by its description and
location: an unreachable assertion in the harness is then an `error`, one in code the harness
calls (or in a contract it uses) a `warning`, and a built-in or library check `info`.
`check-specs.py --probe-harnesses` decides with Kani whether the harness end is reachable.

Usage:
    cargo kani -Z export-json --export-json results.json ... | tee kani.log
    check-vacuity.py results.json [--log kani.log]

Exit status: 0 when nothing is suspicious (only `info` findings), 2 when at least one
`warning` or `error` finding is reported, 1 on input errors.
"""

import argparse
import json
import re
import sys

# Property classes that come from user-written code rather than Kani's default safety checks.
USER_ASSERTION_CLASSES = {"assertion", "assert", "kani::assert"}


# Descriptions of checks the Rust compiler and Kani insert, as opposed to user assertions.
BUILT_IN = re.compile(r'^"?(attempt to |index out of bounds|misaligned pointer|null pointer|unreachable code'
                      r'|`async fn` resumed|dereference failure|pointer arithmetic|arithmetic overflow)')
# Property names can contain spaces (`<impl Foo for Bar>::f.assertion.1`, `{closure@...}`).
LOG_CHECK = re.compile(r'^Check \d+: (.+?)[ \t]*\n[ \t]*- Status: (\w+)[ \t]*\n[ \t]*- Description: (.*?)[ \t]*$'
                       r'(?:\n[ \t]*- Location: (.*?)[ \t]*$)?', re.M)
# Checks located in Kani's library or the Rust standard library are not user assertions.
LIBRARY_FILE = re.compile(r"(^|/)(library/(core|std|alloc|kani|kani_core)/|kani_core/|\.kani/|rustlib/|"
                          r"/rustc/|kani/library/)")


def parse_log(text):
    """Property ID -> {description, file, function}, from Kani's regular console output."""
    out = {}
    for m in LOG_CHECK.finditer(text):
        entry = {"description": m.group(3).strip()}
        loc = m.group(4)
        if loc:
            where, _, function = loc.partition(" in function ")
            entry["file"] = where.rsplit(":", 2)[0] if where.count(":") >= 2 else where
            entry["function"] = function.strip() or None
        out[m.group(1).strip()] = entry
    return out


def in_harness(function, harness):
    """Whether a check located in `function` is code of the harness itself (or a closure in it)."""
    if not function:
        return False
    target = harness.split("::")[-1]
    return target in re.split(r"::|\{|\}", function)


def prop_class(prop_id):
    # Property IDs have the shape `<function>.<class>.<index>`.
    parts = prop_id.rsplit(".", 2)
    return parts[1] if len(parts) == 3 else ""


def check_harness(harness, descriptions=None):
    findings = []
    name = harness["name"]
    checks = harness.get("checks", {})
    covers = harness.get("covers", {})

    unreachable_asserts = [
        pid for pid in checks.get("unreachable", []) if prop_class(pid) in USER_ASSERTION_CLASSES
    ]
    for pid in unreachable_asserts:
        info = (descriptions or {}).get(pid)
        description = info["description"] if info else None
        if info is not None and (BUILT_IN.match(description) or LIBRARY_FILE.search(info.get("file") or "")):
            findings.append({
                "harness": name, "severity": "info", "rule": "unreachable-builtin-check", "property": pid,
                "description": description,
                "message": "a built-in or library check is unreachable: the solver showed it cannot fail. "
                           "Not a vacuity issue.",
            })
        elif info is not None and in_harness(info.get("function"), name):
            findings.append({
                "harness": name, "severity": "error", "rule": "unreachable-assertion", "property": pid,
                "description": description,
                "message": "an assertion in the harness is never reached; it holds vacuously. Check the "
                           "harness assumptions and the path to this assertion.",
            })
        elif info is not None:
            findings.append({
                "harness": name, "severity": "warning", "rule": "unreachable-callee-assertion", "property": pid,
                "description": description, "function": info.get("function"),
                "message": f"an assertion in `{info.get('function')}` (code the harness calls, or a contract it "
                "uses) is never reached: the harness does not exercise that path. Fine if the "
                "property does not need it; otherwise widen the inputs.",
            })
        else:
            findings.append({
                "harness": name, "severity": "warning", "rule": "unreachable-assertion", "property": pid,
                "message": "an assertion-class check is never reached. It may be a user assertion (vacuous) "
                           "or a built-in overflow/division check the solver proved cannot fail. Re-run "
                           "with `--log` to tell them apart, and use `check-specs.py --probe-harnesses`.",
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
            "severity": "warning",
            "rule": "all-checks-unreachable",
            "message": "every check in this harness is unreachable: the checked code may be cut off by an "
                       "assumption, an early return, or a panic, or every check may be a built-in check the "
                       "solver folded away. Decide with `check-specs.py --probe-harnesses` and `--log`.",
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


def check(doc, descriptions=None):
    findings = []
    skipped = []
    for harness in doc.get("harnesses", []):
        outcome = harness.get("outcome", {})
        if outcome.get("kind") != "COMPLETED" or outcome.get("verdict") != "SUCCESS":
            skipped.append(harness["name"])
            continue
        findings.extend(check_harness(harness, descriptions))
    return {"findings": findings, "skipped_unsuccessful": skipped}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("export_json", help="file written by `--export-json`")
    parser.add_argument("--log", help="Kani's console output for the same run (regular output format)")
    args = parser.parse_args(argv)
    try:
        with open(args.export_json, encoding="utf-8") as f:
            doc = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"error: cannot read Kani JSON export `{args.export_json}`: {e}", file=sys.stderr)
        return 1
    descriptions = None
    if args.log:
        try:
            descriptions = parse_log(open(args.log, encoding="utf-8", errors="replace").read())
        except OSError as e:
            print(f"error: cannot read Kani log `{args.log}`: {e}", file=sys.stderr)
            return 1
    result = check(doc, descriptions)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 2 if any(f["severity"] != "info" for f in result["findings"]) else 0


if __name__ == "__main__":
    sys.exit(main())
