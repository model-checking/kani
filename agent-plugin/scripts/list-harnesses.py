#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""List the harnesses and contracts of a Rust project, without leaving files behind.

`cargo kani list --format json` has rough edges that every agent runs into: it writes
`kani-list.json` into the current directory (and cannot be told otherwise), it rejects
`--target-dir` and `--features`, it needs the unstable features the code uses (`-Z
function-contracts`, `-Z stubbing`), it prints nothing but a generic message when the crate does not
compile, and it prints a spurious "unstable feature" warning. This script runs it, moves the JSON
to the plugin output directory (`--out`), deletes the clutter if it created it, adds the unstable
features it detects in the sources, and, when there is no JSON, shows the compiler's errors.

Usage:
    list-harnesses.py [PATH] [--out FILE] [-Z FEATURE]... [--full]

PATH is the Cargo project directory (default: the current one). By default the output is a
summary (counts per file, contracts without a harness); `--full` prints the whole listing. Use
`CARGO_TARGET_DIR` to move cargo's target directory: `cargo kani list` has no `--target-dir`.

Exit status: 0 on success, 1 when Kani produced no listing (the output then holds the errors).
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import emit, find_root, kani_env, plugin_out_dir, rust_files  # noqa: E402

AUTO_FEATURES = [
    ("function-contracts", re.compile(r"kani::(requires|ensures|modifies|proof_for_contract|stub_verified)")),
    ("stubbing", re.compile(r"kani::stub\b")),
    ("loop-contracts", re.compile(r"kani::(loop_invariant|loop_modifies)")),
]
NOISE = re.compile(r"use of an unstable feature|register_tool|requested on the command line with `--force-warn|"
                   r"^\s*-->\s*<crate attribute>|^\s*\|\s*$|^\s*\d+ \| #!\[feature")


def detect_features(root):
    found = []
    text = "\n".join(f.read_text(encoding="utf-8", errors="replace") for f in rust_files([root]))
    for name, pattern in AUTO_FEATURES:
        if pattern.search(text):
            found.append(name)
    return found


def compiler_errors(text, limit=6):
    """The first compiler error blocks in `text`."""
    errors, current = [], None
    for line in text.splitlines():
        if re.match(r"^error(\[E\d+\])?:", line):
            current = [line]
            errors.append(current)
        elif current is not None and (line.startswith(" ") or line.startswith("-->")) and len(current) < 8:
            current.append(line)
        else:
            current = None
    return ["\n".join(e) for e in errors[:limit]]


def summarize(listing, out_path):
    def per_file(key):
        return {f: len(names) for f, names in listing.get(key, {}).items()}
    without = [c["function"] for c in listing.get("contracts", []) if not c.get("harnesses")]
    return {"kani_version": listing.get("kani-version"), "totals": listing.get("totals"),
            "standard_harnesses_per_file": per_file("standard-harnesses"),
            "contract_harnesses_per_file": per_file("contract-harnesses"),
            "contracts_without_harness": without,
            "listing": str(out_path)}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("path", nargs="?", default=".", help="Cargo project directory (default: .)")
    parser.add_argument("--out", help="where to write the listing (default: <plugin output>/kani-list.json)")
    parser.add_argument("-Z", dest="unstable", action="append", default=[], metavar="FEATURE",
                        help="unstable Kani feature to enable (repeatable); detected ones are added")
    parser.add_argument("--full", action="store_true", help="print the whole listing, not a summary")
    args = parser.parse_args(argv)
    cwd = Path(args.path).resolve()
    root, package = find_root(cwd)
    if package is None:
        print("error: not a Cargo project; for a single file run `kani list FILE.rs`", file=sys.stderr)
        return 1
    features = list(dict.fromkeys([*args.unstable, *detect_features(package)]))
    flags = [x for f in features for x in ("-Z", f)]
    clutter = package / "kani-list.json"
    existed = clutter.exists()
    out_path = Path(args.out) if args.out else plugin_out_dir(root) / "kani-list.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    run = subprocess.run(["cargo", "kani", "list", "--format", "json", *flags], cwd=package,
                         capture_output=True, text=True, env=kani_env())
    listing = None
    if clutter.exists():
        try:
            listing = json.loads(clutter.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            pass
        out_path.write_text(clutter.read_text(encoding="utf-8"), encoding="utf-8") if listing else None
        if not existed:
            clutter.unlink()
    if listing is None:
        # No listing: the crate probably does not compile. The plain list shows the diagnostics.
        plain = subprocess.run(["cargo", "kani", "list", *flags], cwd=package, capture_output=True, text=True,
                               env=kani_env())
        text = "\n".join(ln for ln in (run.stderr + "\n" + plain.stdout + "\n" + plain.stderr).splitlines()
                         if not NOISE.search(ln))
        errors = compiler_errors(text)
        emit({"error": "`cargo kani list` produced no listing", "unstable_features_used": features,
              "compiler_errors": errors,
              "output_tail": "\n".join(text.splitlines()[-15:]) if not errors else None,
              "hint": "Fix the errors above. If Kani rejected an option, `cargo kani list` takes only "
                      "`--format` and `-Z <feature>`: use CARGO_TARGET_DIR instead of --target-dir, and build "
                      "features with the crate's default features or edit Cargo.toml."})
        for leftover in (clutter,):
            if leftover.exists() and not existed:
                leftover.unlink()
        return 1
    if args.full:
        listing["listing_path"] = str(out_path)
        emit(listing)
    else:
        emit(summarize(listing, out_path))
    return 0


if __name__ == "__main__":
    sys.exit(main())
