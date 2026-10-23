#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Compact summary of a Kani `--export-json` document (RFC 0015).

Produces only the facts an agent needs to act on a verification run: per-harness verdict,
the failed properties with their source locations, and the counts that matter for proof
quality (unreachable checks, unsatisfiable covers, unsupported constructs). It also reports
what a SUCCESS relies on: `weakened_checks` lists options of the run that disable checks, and
each harness's `trusted` lists its stubs and `stub_verified` targets. Empty fields are omitted
to keep the output small.

Usage:
    cargo kani -Z export-json --export-json results.json ...
    summarize-results.py results.json [--failures-only] [--max-failed N]

The `kani_verify_harness` MCP tool returns the same shape.
"""

import argparse
import json
import sys


def verdict_of(harness):
    outcome = harness.get("outcome", {})
    if outcome.get("kind") == "COMPLETED":
        return outcome.get("verdict", "UNKNOWN")
    return outcome.get("kind", "UNKNOWN")


def location(prop):
    if prop.get("file") is None:
        return None
    return f"{prop['file']}:{prop['line']}" if prop.get("line") is not None else prop["file"]


# Each entry: (configuration.checks key, value that weakens the proof, the flag that set it).
WEAKENING_CHECKS = [
    ("memory_safety", False, "--no-memory-safety-checks"),
    ("overflow", False, "--no-overflow-checks"),
    ("unwinding", False, "--no-unwinding-checks"),
    ("undefined_function", False, "--no-undefined-function-checks"),
    ("assertion_reach_checks", False, "--no-assertion-reach-checks"),
    ("assert_contracts", False, "--no-assert-contracts"),
    ("prove_safety_only", True, "--prove-safety-only"),
    ("ignore_global_asm", True, "--ignore-global-asm"),
]


def weakened_checks(doc):
    """The Kani options of this run that disable checks or change what a SUCCESS means."""
    config = doc.get("configuration", {})
    checks = config.get("checks", {})
    weakened = [flag for key, bad, flag in WEAKENING_CHECKS if checks.get(key) is bad]
    if config.get("cbmc_args"):
        weakened.append("--cbmc-args " + " ".join(config["cbmc_args"]))
    return weakened


def trusted_constructs(harness):
    """What the harness's result relies on besides the code itself, as recorded by Kani."""
    attributes = harness.get("attributes", {})
    trusted = {
        "stubs": [f"{s['original']} -> {s['replacement']}" for s in attributes.get("stubs", [])],
        "verified_stubs": list(attributes.get("verified_stubs", [])),
    }
    return {k: v for k, v in trusted.items() if v}


def summarize_harness(harness, max_failed):
    out = {
        "name": harness["name"],
        "verdict": verdict_of(harness),
        "location": f"{harness['file']}:{harness['line']}",
        "time_s": round(harness.get("resources", {}).get("verification_time_s", 0.0), 2),
    }
    outcome = harness.get("outcome", {})
    if outcome.get("message"):
        out["message"] = outcome["message"]
    failed = harness.get("failed_properties", [])
    if failed:
        out["failed"] = [
            {k: v for k, v in (
                ("class", p["class"]),
                ("description", p["description"]),
                ("location", location(p)),
            ) if v is not None}
            for p in failed[:max_failed]
        ]
        if len(failed) > max_failed:
            out["failed_omitted"] = len(failed) - max_failed
    checks = harness.get("checks", {})
    covers = harness.get("covers", {})
    counts = {
        "checks_total": checks.get("total"),
        "unreachable_checks": len(checks.get("unreachable", [])),
        "undetermined_checks": len(checks.get("undetermined", [])),
        "covers_total": covers.get("total"),
        "unsatisfiable_covers": len(covers.get("unsatisfiable", [])),
        "unreachable_covers": len(covers.get("unreachable", [])),
        "unsupported_constructs": len(harness.get("unsupported_constructs", [])),
        "warnings": len(harness.get("warnings", [])) + harness.get("warnings_truncated", 0),
    }
    out.update({k: v for k, v in counts.items() if v})
    flags = [
        name for name, on in (
            ("automatically_generated", harness.get("is_automatically_generated")),
            ("bounded", harness.get("is_bounded")),
            ("contract", harness.get("contract") is not None),
            ("should_panic", harness.get("attributes", {}).get("should_panic")),
            ("concrete_test_generated", harness.get("generated_concrete_test")),
        ) if on
    ]
    if flags:
        out["flags"] = flags
    trusted = trusted_constructs(harness)
    if trusted:
        out["trusted"] = trusted
    return out


def summarize(doc, failures_only=False, max_failed=5):
    harnesses = [summarize_harness(h, max_failed) for h in doc.get("harnesses", [])]
    if failures_only:
        harnesses = [h for h in harnesses if h["verdict"] != "SUCCESS"]
    summary = doc.get("summary", {})
    out = {
        "kani": doc.get("tools", {}).get("kani"),
        "run_state": doc.get("run_state"),
        "total": summary.get("total", 0),
        "successful": summary.get("successful", 0),
        "failed": summary.get("failed", 0),
        "harnesses": harnesses,
    }
    weakened = weakened_checks(doc)
    if weakened:
        out["weakened_checks"] = weakened
    unmatched = doc.get("harness_selection", {}).get("unmatched_filters", [])
    if unmatched:
        out["unmatched_filters"] = unmatched
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("export_json", help="file written by `--export-json`")
    parser.add_argument("--failures-only", action="store_true", help="omit successful harnesses")
    parser.add_argument("--max-failed", type=int, default=5, help="failed properties shown per harness")
    args = parser.parse_args(argv)
    try:
        with open(args.export_json, encoding="utf-8") as f:
            doc = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"error: cannot read Kani JSON export `{args.export_json}`: {e}", file=sys.stderr)
        return 1
    print(json.dumps(summarize(doc, args.failures_only, args.max_failed), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
