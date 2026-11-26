#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Clean up the tests that `--concrete-playback=inplace` adds to a source file.

Kani's inplace playback appends one test per failing check on every run, so a second run adds the
same tests again (the file then fails to compile with duplicate function names), and a leftover
doc comment can be left without an item. This script keeps the first copy of each generated
`kani_concrete_playback_*` test and removes later duplicates and dangling generated doc comments.

Usage:
    cleanup-playback.py FILE.rs... [--dry-run]

It never touches anything that is not a generated playback test. Tests that need `vec!`/`Vec` in a
`#![no_std]` crate need `extern crate std; use std::{vec, vec::Vec};` in their module: `check-witness.py`
adds that in its work copy; add it yourself if you keep the tests.
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import parse_functions  # noqa: E402

GENERATED = re.compile(r"kani_concrete_playback_")
DOC_START = "/// Test generated for harness"
DANGLING = re.compile(r"(?m)^[ \t]*///[^\n]*\n(?:[ \t]*///[^\n]*\n)*(?:[ \t]*\n)*(?=[ \t]*\}|\Z)")


def clean(src):
    """Returns (new source, names removed)."""
    removed, ranges, seen = [], [], set()
    for fn in sorted(parse_functions(src), key=lambda f: f.start):
        if not GENERATED.match(fn.name):
            continue
        if fn.name not in seen:
            seen.add(fn.name)
            continue
        start = src.rfind(DOC_START, 0, fn.start)
        block = src[start:fn.start] if start != -1 else ""
        if start == -1 or re.search(r"\bfn\s", block):
            start = fn.start
        ranges.append((start, fn.end))
        removed.append(fn.name)
    for start, end in reversed(ranges):
        src = src[:start] + src[end:]
    # Dangling generated doc comments (a doc comment left without an item).
    new = DANGLING.sub(lambda m: "" if DOC_START in m.group(0) or "Check for `" in m.group(0) else m.group(0), src)
    return new, removed, len(new) != len(src)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("files", nargs="+")
    parser.add_argument("--dry-run", action="store_true", help="report what would be removed; change nothing")
    args = parser.parse_args(argv)
    report = []
    for name in args.files:
        path = Path(name)
        src = path.read_text(encoding="utf-8")
        new, removed, dangling = clean(src)
        if new != src and not args.dry_run:
            path.write_text(new, encoding="utf-8")
        report.append({"file": name, "duplicates_removed": removed, "dangling_doc_comments_removed": dangling,
                       "changed": new != src, "dry_run": args.dry_run})
    print(json.dumps({"files": report}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
