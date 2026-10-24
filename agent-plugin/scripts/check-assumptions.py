#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Flag suspicious `kani::assume` and `#[kani::requires]` usage in Rust sources.

Every assumption narrows the set of inputs a proof covers. Over-constraining assumptions
are the most common way a Kani proof ends up proving much less than intended, or nothing
at all. This is a static, text-based scan: it needs no toolchain and reports suspicions for
review, not verdicts. Pair it with `check-vacuity.py`, which checks what Kani actually
reached.

Rules:
- assume-false / requires-false: the condition is literally `false`; everything after it
  is vacuous.
- contradictory-bounds: bounds on the same variable within one harness cannot all hold
  (e.g. `x > 10` and `x < 5`).
- pinned-value: `x == <literal>` reduces the proof to a single test case for `x`.
- assume-success: assuming a call returns `Ok`/`Some` hides the error path.
- assume-after-assert: an assumption that follows an assertion filters the paths the later
  assertions are checked on.
- many-assumptions: more than 5 assumptions in one harness; review whether each is needed.

Usage:
    check-assumptions.py PATH... [--harness NAME]

Exit status: 0 when nothing is suspicious (only `info` findings), 2 when at least one
`warning` or `error` finding is reported, 1 on input errors.
"""

import argparse
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import (  # noqa: E402
    emit, find_root, harnesses_of, helper_closure, helper_index, kani_only_files, line_of, parse_functions,
    rust_files, strip_comments_and_strings)

ASSUME = re.compile(r"\bkani::assume\s*\(")
ASSERT = re.compile(
    r"\b(?:assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne)!\s*\(|\bkani::assert\s*\(")
REQUIRES = re.compile(r"#\[\s*(?:cfg_attr\s*\(\s*kani\s*,\s*)?kani::requires\s*\(")
NUM = r"-?(?:0x[0-9a-fA-F_]+|0b[01_]+|[0-9][0-9_]*)(?:[iu](?:8|16|32|64|128|size))?"
BOUND = re.compile(rf"\b([A-Za-z_][A-Za-z0-9_.]*)\s*(<=|>=|==|<|>)\s*({NUM})\b")
REV_BOUND = re.compile(rf"(?<![A-Za-z0-9_.])({NUM})\s*(<=|>=|==|<|>)\s*([A-Za-z_][A-Za-z0-9_.]*)\b")
SUCCESS = re.compile(r"\.(is_ok|is_some)\s*\(\s*\)")
PRIMITIVES = {f"{p}{w}" for p in "iu" for w in ("8", "16", "32", "64", "128", "size")}
FLIP = {"<": ">", ">": "<", "<=": ">=", ">=": "<=", "==": "=="}
MANY_ASSUMPTIONS = 5


def call_args(code, open_paren):
    """Return the text between `open_paren` and its matching `)`."""
    depth = 0
    for k in range(open_paren, len(code)):
        if code[k] == "(":
            depth += 1
        elif code[k] == ")":
            depth -= 1
            if depth == 0:
                return code[open_paren + 1: k]
    return code[open_paren + 1:]


def parse_int(text):
    t = re.sub(r"[iu](?:8|16|32|64|128|size)$", "", text).replace("_", "")
    return int(t, 0)


def bounds_in(cond):
    """Conjunctive bounds `var op literal` in a condition. Conditions containing `||` or `!`
    are skipped: they are not plain conjunctions of bounds."""
    if "||" in cond or re.search(r"!(?!=)", cond):
        return []
    found = [(v, op, parse_int(n)) for v, op, n in BOUND.findall(cond)]
    found += [(v, FLIP[op], parse_int(n)) for n, op, v in REV_BOUND.findall(cond)]
    # `x as u32 < 5` would otherwise bind the bound to the type name.
    return [b for b in found if b[0] not in PRIMITIVES]


def interval_empty(constraints):
    lo, hi = float("-inf"), float("inf")
    for op, n in constraints:
        if op == "<":
            hi = min(hi, n - 1)
        elif op == "<=":
            hi = min(hi, n)
        elif op == ">":
            lo = max(lo, n + 1)
        elif op == ">=":
            lo = max(lo, n)
        else:
            lo, hi = max(lo, n), min(hi, n)
    return lo > hi


def finding(file, line, harness, severity, rule, message):
    return {"file": file, "line": line, "harness": harness, "severity": severity,
            "rule": rule, "message": message}


def check_harness(h, helpers=()):
    """Check the assumptions of harness `h`, including those in the Kani-only `helpers` it
    calls (see `kani_plugin_lib.helper_closure`)."""
    findings = []
    per_var = {}
    assumes = []
    for unit in [h, *helpers]:
        first_assert = ASSERT.search(unit.body) if unit is h else None
        for m in ASSUME.finditer(unit.body):
            assumes.append((unit, m, first_assert))
    for unit, m, first_assert in assumes:
        body = unit.body
        cond = call_args(body, m.end() - 1).strip()
        line = unit.body_line + body.count("\n", 0, m.start())
        h_file = unit.file
        if cond == "false":
            findings.append(finding(h_file, line, h.name, "error", "assume-false",
                                    "`kani::assume(false)` makes every later check vacuous."))
            continue
        if cond == "true":
            findings.append(finding(h_file, line, h.name, "info", "assume-true",
                                    "`kani::assume(true)` has no effect; remove it."))
            continue
        for var, op, n in bounds_in(cond):
            per_var.setdefault(var, []).append((op, n, line))
            if op == "==":
                findings.append(finding(
                    h_file, line, h.name, "warning", "pinned-value",
                    f"`{var} == {n}` pins `{var}` to one value; the proof covers a single case "
                    "for it. Prefer a range, or drop the assumption."))
        if SUCCESS.search(cond):
            findings.append(finding(
                h_file, line, h.name, "warning", "assume-success",
                "assuming a call succeeds (`is_ok`/`is_some`) hides its error path; check that "
                "the failure case is verified elsewhere or is genuinely impossible."))
        if first_assert and m.start() > first_assert.start():
            findings.append(finding(
                h_file, line, h.name, "warning", "assume-after-assert",
                "assumption after an assertion: assertions below it are only checked on the "
                "paths it allows. Move input constraints to the top of the harness."))
    for var, cs in per_var.items():
        if interval_empty([(op, n) for op, n, _ in cs]):
            findings.append(finding(
                h.file, cs[-1][2], h.name, "error", "contradictory-bounds",
                f"the assumptions on `{var}` ({', '.join(f'{op} {n}' for op, n, _ in cs)}) cannot "
                "all hold; the harness is vacuous."))
    if len(assumes) > MANY_ASSUMPTIONS:
        findings.append(finding(
            h.file, h.line, h.name, "info", "many-assumptions",
            f"{len(assumes)} assumptions; review whether each one is required by the code "
            "under verification."))
    return findings


def check_requires(path):
    src = Path(path).read_text(encoding="utf-8", errors="replace")
    code = strip_comments_and_strings(src)
    findings = []
    for m in REQUIRES.finditer(code):
        cond = call_args(code, m.end() - 1).strip()
        if cond == "false":
            findings.append(finding(str(path), line_of(code, m.start()), None, "error",
                                    "requires-false",
                                    "`#[kani::requires(false)]` makes the contract vacuous."))
        else:
            per_var = {}
            for var, op, n in bounds_in(cond):
                per_var.setdefault(var, []).append((op, n))
            for var, cs in per_var.items():
                if interval_empty(cs):
                    findings.append(finding(
                        str(path), line_of(code, m.start()), None, "error", "contradictory-bounds",
                        f"the precondition bounds on `{var}` cannot all hold; the contract is vacuous."))
    return findings


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("paths", nargs="+", help="Rust files or directories to scan")
    parser.add_argument("--harness", action="append", default=[], help="only check these harnesses")
    args = parser.parse_args(argv)
    findings = []
    n_harnesses = 0
    try:
        targets = list(rust_files(args.paths))
        _, package = find_root(targets[0]) if targets else (None, None)
        crate = list(rust_files([package])) if package else targets
        kani_only = kani_only_files(crate)
        parsed = {f.resolve(): parse_functions(f.read_text(encoding="utf-8", errors="replace"), f,
                                               f.resolve() in kani_only) for f in {*crate, *targets}}
        index = helper_index([fn for fns in parsed.values() for fn in fns])
        for f in targets:
            for fn in parsed[f.resolve()]:
                hs = harnesses_of([fn])
                if not hs or (args.harness and fn.name not in args.harness):
                    continue
                n_harnesses += 1
                findings.extend(check_harness(hs[0], helper_closure(fn, index)))
            findings.extend(check_requires(f))
    except OSError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    emit({"harnesses_checked": n_harnesses, "findings": findings})
    return 2 if any(f["severity"] != "info" for f in findings) else 0


if __name__ == "__main__":
    sys.exit(main())
