#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Compact summary of a Kani `--export-json` document (RFC 0015).

Produces only the facts an agent needs to act on a verification run: per-harness verdict,
the failed properties with their source locations, and the counts that matter for proof
quality (unreachable checks, unsatisfiable covers, unsupported constructs). It also reports
what a SUCCESS relies on: `weakened_checks` lists options of the run that disable checks, and
each harness's `trusted` lists its stubs and `stub_verified` targets.

Every count key is always present (zero included), so scripts can index them directly.
`failed` counts harnesses that failed; `inconclusive` counts those that timed out, ran out of
memory, or crashed, which is not the same as failing. `all_ok` is true only when every harness
succeeded. Unsupported constructs, which make Kani's result unreliable, are listed per harness.

With `--log kani.log` (Kani's console output of the same run), each harness that did not
succeed gets a `diagnosis`: the log often shows the real cause of a TIMEOUT or OUT_OF_MEMORY
(an unwinding bound that is too small, unbounded recursion, a solver timeout).

Usage:
    cargo kani -Z export-json --export-json results.json ... | tee kani.log
    summarize-results.py results.json [--failures-only] [--max-failed N] [--log kani.log]

When the export does not exist (the run was killed, or failed to compile), the script says so and,
with `--log`, reports the verdicts that the log does show.

The `kani_verify_harness` MCP tool returns the same shape.
"""

import argparse
import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_export import ExportError, NO_CONFIGURATION_NOTE, load_export, weakened_checks_detectable  # noqa: E402

INCONCLUSIVE = ("TIMEOUT", "OUT_OF_MEMORY", "CRASHED")


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
    trusted = {k: v for k, v in trusted.items() if v}
    source = attributes.get("stubs_source", "export")
    if trusted and source != "export":
        # The export itself did not list them: they were read from the harness source.
        trusted["stubs_source"] = source
    elif source == "unknown":
        trusted["stubs_source"] = "unknown: the export does not list stubs and the harness source was not found"
    return trusted


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
    unsupported = harness.get("unsupported_constructs", [])
    out.update({
        "checks_total": checks.get("total") or 0,
        "unreachable_checks": len(checks.get("unreachable", [])),
        "undetermined_checks": len(checks.get("undetermined", [])),
        "covers_total": covers.get("total") or 0,
        "unsatisfiable_covers": len(covers.get("unsatisfiable", [])),
        "unreachable_covers": len(covers.get("unreachable", [])),
        "unsupported_constructs": len(unsupported),
        "warnings": len(harness.get("warnings", [])) + harness.get("warnings_truncated", 0),
    })
    if unsupported:
        out["unsupported"] = [
            {k: v for k, v in (("description", p.get("description")), ("location", location(p))) if v}
            for p in unsupported[:max_failed]]
    out["flags"] = [
        name for name, on in (
            ("automatically_generated", harness.get("is_automatically_generated")),
            ("bounded", harness.get("is_bounded")),
            ("contract", harness.get("contract") is not None),
            ("should_panic", harness.get("attributes", {}).get("should_panic")),
            ("concrete_test_generated", harness.get("generated_concrete_test")),
        ) if on
    ]
    trusted = trusted_constructs(harness)
    if trusted:
        out["trusted"] = trusted
    return out


def summarize(doc, failures_only=False, max_failed=5, diagnoses=None):
    harnesses = [summarize_harness(h, max_failed) for h in doc.get("harnesses", [])]
    if diagnoses:
        for h in harnesses:
            d = diagnoses.get(h["name"]) or diagnoses.get(h["name"].split("::")[-1])
            if d and h["verdict"] != "SUCCESS":
                h["diagnosis"] = d
    verdicts = {}
    for h in harnesses:
        verdicts[h["verdict"]] = verdicts.get(h["verdict"], 0) + 1
    if failures_only:
        harnesses = [h for h in harnesses if h["verdict"] != "SUCCESS"]
    summary = doc.get("summary", {})
    total = summary.get("total", sum(verdicts.values()))
    out = {
        "kani": doc.get("tools", {}).get("kani"),
        "run_state": doc.get("run_state"),
        "started_at": doc.get("started_at"),
        "total": total,
        "successful": verdicts.get("SUCCESS", 0),
        "failed": verdicts.get("FAILURE", 0),
        "inconclusive": sum(n for v, n in verdicts.items() if v in INCONCLUSIVE),
        "by_verdict": dict(sorted(verdicts.items())),
        "all_ok": bool(total) and verdicts.get("SUCCESS", 0) == total,
        "harnesses": harnesses,
    }
    unmatched = doc.get("harness_selection", {}).get("unmatched_filters", [])
    if unmatched:
        out["unmatched_filters"] = unmatched
    weakened = weakened_checks(doc)
    if weakened:
        out["weakened_checks"] = weakened
    out["export_shape"] = doc.get("export_shape", "flat")
    out["weakened_checks_detectable"] = weakened_checks_detectable(doc)
    if not out["weakened_checks_detectable"]:
        out["weakened_checks_note"] = NO_CONFIGURATION_NOTE
    return out


# ---- Console log ----------------------------------------------------------------------

START = re.compile(
    r"^(?:Thread \d+: )?(?:Checking harness (.+?)\.\.\.|Autoharness: Checking function (.+?)(?:'s contract)? against)")
PREFIX = re.compile(r"^Thread (\d+): ")


def split_blocks(text):
    """Lines of the log grouped by harness (sequential or `Thread N:` parallel output)."""
    blocks, current = {}, {}
    for raw in text.splitlines():
        m = PREFIX.match(raw)
        key = m.group(1) if m else ""
        line = raw[m.end():] if m else raw
        s = START.match(raw)
        if s:
            current[key] = (s.group(1) or s.group(2)).strip()
            blocks.setdefault(current[key], [])
            continue
        if key in current:
            blocks[current[key]].append(line)
    return blocks


CAUSES = [
    ("unwind-bound", re.compile(r"unwinding assertion|Failed Checks: unwinding"),
     "an unwinding bound is too small for a loop: add or raise `#[kani::unwind(N)]` (N must exceed the "
     "maximum number of iterations), or bound the input that drives the loop"),
    ("recursion", re.compile(r"Not unwinding recursion|recursion"),
     "Kani hit unbounded recursion (often drop glue of a recursive type): bound the recursion with "
     "`#[kani::unwind]`, give the function a contract and use `stub_verified`, or `core::mem::forget` the "
     "value if dropping is not part of the property (and say so in the report)"),
    ("timeout", re.compile(r"timed out|CBMC timed out"),
     "the solver timed out: shrink input bounds and unwinding, stub expensive callees, or split the harness "
     "(references/kani.md, Performance)"),
    ("out-of-memory", re.compile(r"out of memory|OUT_OF_MEMORY|Cannot allocate memory", re.I),
     "CBMC ran out of memory: shrink symbolic inputs first (a `char`, `String`, or slice multiplies the "
     "state), lower unwind bounds, or stub callees"),
    ("cbmc-crash", re.compile(r"CBMC failed|exited with status 15|status 15"),
     "CBMC crashed; this is usually memory exhaustion or an unwinding explosion: see the other causes"),
]


def diagnose_log(text):
    """harness name -> {cause, hint, evidence}, from Kani's console output."""
    out = {}
    for name, lines in split_blocks(text).items():
        body = "\n".join(lines)
        for cause, pattern, hint in CAUSES:
            m = pattern.search(body)
            if m:
                out[name] = {"cause": cause, "hint": hint,
                             "evidence": next((ln.strip() for ln in lines if pattern.search(ln)), m.group(0))[:160]}
                break
    return out


def verdicts_from_log(text):
    """Verdicts the console log shows, for runs that wrote no export."""
    results, current, failed = [], {}, {}
    for raw in text.splitlines():
        m = PREFIX.match(raw)
        key = m.group(1) if m else ""
        line = raw[m.end():] if m else raw
        s = START.match(raw)
        if s:
            current[key] = (s.group(1) or s.group(2)).strip()
            failed.pop(key, None)
        elif line.startswith("Failed Checks: ") and key in current:
            failed.setdefault(key, []).append(line[len("Failed Checks: "):].strip())
        elif line.startswith("VERIFICATION:- ") and key in current:
            verdict = {"SUCCESSFUL": "SUCCESS", "FAILED": "FAILURE"}.get(line.split("- ", 1)[1].strip(), "UNKNOWN")
            entry = {"name": current[key], "verdict": verdict}
            if key in failed:
                entry["failed"] = failed.pop(key)[:5]
            results.append(entry)
    done = {r["name"] for r in results}
    return results, [n for n in dict.fromkeys(current.values()) if n not in done]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("export_json", help="file written by `--export-json`")
    parser.add_argument("--failures-only", action="store_true", help="omit successful harnesses")
    parser.add_argument("--max-failed", type=int, default=5, help="failed properties shown per harness")
    parser.add_argument("--workspace", help="the project directory, to find harness sources (stubs of older "
                        "Kani exports are read from them; default: the current directory)")
    parser.add_argument("--log", help="Kani's console output of the same run, to diagnose non-successes")
    args = parser.parse_args(argv)
    log_text = None
    if args.log:
        try:
            log_text = open(args.log, encoding="utf-8", errors="replace").read()
        except OSError as e:
            print(f"error: cannot read Kani log `{args.log}`: {e}", file=sys.stderr)
            return 1
    if not os.path.exists(args.export_json):
        msg = {"error": f"no Kani export at `{args.export_json}`",
               "why": "Kani writes `--export-json` only when a run completes. A run that was killed (a time "
                      "limit, the memory watchdog, a closed terminal) or that failed to compile leaves none. "
                      "Re-run with `scripts/run-harnesses.py` (one harness per run, results kept as they finish), "
                      "or with the MCP tools and `timeout_s`."}
        if log_text is not None:
            finished, unfinished = verdicts_from_log(log_text)
            msg["verdicts_in_log"] = finished
            msg["unfinished"] = unfinished
            msg["diagnoses"] = diagnose_log(log_text)
        print(json.dumps(msg, indent=2, sort_keys=True))
        return 1
    try:
        doc = load_export(args.export_json, workspace=args.workspace)
    except ExportError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    out = summarize(doc, args.failures_only, args.max_failed, diagnose_log(log_text) if log_text else None)
    out["export_age_minutes"] = round((time.time() - os.path.getmtime(args.export_json)) / 60, 1)
    try:
        print(json.dumps(out, indent=2, sort_keys=True))
        sys.stdout.flush()
    except BrokenPipeError:
        os._exit(0)
    return 0


if __name__ == "__main__":
    sys.exit(main())
