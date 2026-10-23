#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Describe the Kani harnesses in Rust sources, and flag structural problems.

For each harness, reports its kind (`proof` / `proof_for_contract`), Kani attributes
(unwind, stubs, solver, should_panic, ...), how many nondeterministic inputs, assumptions,
assertions, and covers it has, and the functions it calls. Flags:

- no-nondeterminism: no `kani::any()` (or `any_where`); the harness checks one concrete
  execution, like a unit test, not all inputs.
- no-assertions: no user assertion or cover; only Kani's default safety checks apply.
  That is fine for UB/panic-freedom proofs, but say so when reporting the result.
- loop-without-unwind: the harness body has a loop but no `#[kani::unwind]`; verification
  may not terminate, or need `--default-unwind`.
- should-panic: the harness passes only if it panics; report it that way.

This is a static, text-based scan that needs no toolchain. Use `cargo kani list --format
json` for the authoritative harness list.

Usage:
    inspect-harness.py PATH... [--harness NAME]
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import emit, find_harnesses, rust_files  # noqa: E402

ANY = re.compile(r"\bkani::any(?:_where)?\b|\bany_(?:modifies|slice_of_array)\b|\bkani::vec::any_vec\b")
ASSUME = re.compile(r"\bkani::assume\s*\(")
ASSERT = re.compile(
    r"\b(?:assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne)!\s*\(|\bkani::assert\s*\(")
COVER = re.compile(r"\bkani::cover!\s*\(")
LOOP = re.compile(r"\b(?:for|while|loop)\b")
CALL = re.compile(r"(?<![!\w])((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*)\s*(?:::<[^>]*>)?\s*\(")
NOT_CALLS = {"if", "while", "for", "match", "return", "Some", "Ok", "Err", "Box::new", "loop", "in"}
MAX_CALLS = 20


def describe(h):
    calls = []
    for c in CALL.findall(h.body):
        if c in NOT_CALLS or c.startswith("kani::") or c in calls:
            continue
        calls.append(c)
    counts = {
        "nondet_inputs": len(ANY.findall(h.body)),
        "assumptions": len(ASSUME.findall(h.body)),
        "assertions": len(ASSERT.findall(h.body)),
        "covers": len(COVER.findall(h.body)),
    }
    flags = []
    if counts["nondet_inputs"] == 0 and h.kind == "proof":
        flags.append("no-nondeterminism")
    if counts["assertions"] == 0 and counts["covers"] == 0 and h.kind == "proof":
        flags.append("no-assertions")
    if LOOP.search(h.body) and "unwind" not in h.attributes:
        flags.append("loop-without-unwind")
    if "should_panic" in h.attributes:
        flags.append("should-panic")
    attrs = {k: v for k, v in h.attributes.items() if k not in ("proof", "proof_for_contract")}
    out = {
        "name": h.name,
        "location": f"{h.file}:{h.line}",
        "kind": h.kind,
        "attributes": attrs,
        "counts": counts,
        "calls": calls[:MAX_CALLS],
        "flags": flags,
    }
    if h.target:
        out["contract_target"] = h.target
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("paths", nargs="+", help="Rust files or directories to scan")
    parser.add_argument("--harness", action="append", default=[], help="only describe these harnesses")
    args = parser.parse_args(argv)
    try:
        harnesses = [
            describe(h)
            for f in rust_files(args.paths)
            for h in find_harnesses(f)
            if not args.harness or h.name in args.harness
        ]
    except OSError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    emit({"harnesses": harnesses})
    return 0


if __name__ == "__main__":
    sys.exit(main())
