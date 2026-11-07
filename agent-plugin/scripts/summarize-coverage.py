#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Merge Kani source coverage (`-Z source-coverage --coverage`) across harnesses.

Coverage shows which code the harnesses actually exercise: a region no harness covers is
checked on no execution, and no verdict shows it. Kani writes one `*_kaniraw.json` file per
harness into `target/kani/<target>/kanicov_<timestamp>/`. A region counts as covered when any
harness covers it. This script merges those files and prints, per source file, the covered
and total regions and the uncovered ones (line, end line, function), so an agent can read the
result without the whole console output. It reads the raw files directly, because `kani-cov
summary` can panic on projects with several files.

Run Kani first:
    cargo kani -Z source-coverage --coverage [--harness NAME]... | tee kani.log

Then:
    summarize-coverage.py --log kani.log [--file SUBSTRING] [--max-uncovered N]
    summarize-coverage.py --crate PATH            # the latest run under PATH/target
    summarize-coverage.py DIR... [--compare DIR...] [--src DIR] [--all-files]

`--compare` takes another set of runs (for example before and after adding a harness) and
reports the regions covered only by one of the two sets.

Regions inside the harnesses themselves (for example an error branch of the harness that
never runs) are reported too: a dead branch in a harness is a gap in the proof. Use `--file`
to focus on the code under verification.

Scope: percentages are over the regions Kani instrumented, which are the functions the harnesses
reach, not over the crate. `files_without_regions` lists the source files (under `--src`, by default
`src/`) that no harness touched at all. Dependencies (files outside the workspace, or under the
cargo registry) are left out unless `--all-files` is given. A region is covered if any harness, and
any generic instance of it, covers that source span.

Exit status: 0 on success, 1 on input errors (no coverage data found).
"""

import argparse
import collections
import glob
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import emit, rust_files  # noqa: E402

SAVED = re.compile(r"Coverage results saved to\s+(\S+)")


def dirs_from_log(path):
    text = open(path, encoding="utf-8", errors="replace").read()
    return [m.group(1) for m in SAVED.finditer(text)]


def latest_run(crate):
    """The newest `kanicov_*` directory under `crate`/target, or under a parent workspace."""
    crate = os.path.abspath(crate)
    here = crate
    while True:
        runs = sorted(glob.glob(os.path.join(here, "target", "kani", "*", "kanicov_*")))
        runs = [r for r in runs if os.path.isdir(r)]
        if runs:
            return [max(runs, key=os.path.getmtime)]
        parent = os.path.dirname(here)
        if parent == here:
            return []
        here = parent


DEPENDENCY = re.compile(r"(^/|\.cargo/(registry|git)/|/rustc/|rustlib/|^[A-Za-z]:[\\/])")


def normalize(path):
    """A path without leading `./` and `../`, for matching `--file` filters."""
    return re.sub(r"^(\.{1,2}/)+", "", path.replace("\\", "/"))


def merge(dirs):
    """(file, start, end) -> {"covered", "functions"}, merged over all raw files in `dirs`. A
    region is covered if any harness (and any generic instance) covers that source span.
    Returns (regions, number of files read, unreadable files)."""
    regions = {}
    files, bad = 0, []
    for d in dirs:
        for f in sorted(glob.glob(os.path.join(d, "*_kaniraw.json"))):
            try:
                with open(f, encoding="utf-8") as fh:
                    data = json.load(fh).get("data", {})
            except (OSError, json.JSONDecodeError):
                bad.append(os.path.basename(f))  # e.g. a run killed before it wrote its results
                continue
            files += 1
            for file, rs in data.items():
                for r in rs:
                    key = (file, tuple(r["region"]["start"]), tuple(r["region"]["end"]))
                    entry = regions.setdefault(key, {"covered": False, "functions": set()})
                    entry["covered"] = entry["covered"] or r["status"] == "COVERED"
                    entry["functions"].add(r["function"])
    return regions, files, bad


def wanted(file, file_filter, all_files):
    if not all_files and DEPENDENCY.search(file):
        return False
    return not file_filter or normalize(file_filter) in normalize(file)


def summarize(regions, file_filter="", max_uncovered=20, all_files=False, src_files=()):
    per = collections.defaultdict(lambda: {"covered": 0, "total": 0, "uncovered": []})
    excluded = set()
    for (file, start, end), info in sorted(regions.items()):
        if not wanted(file, file_filter, all_files):
            if DEPENDENCY.search(file):
                excluded.add(file)
            continue
        entry = per[file]
        entry["total"] += 1
        entry["covered"] += info["covered"]
        if not info["covered"]:
            entry["uncovered"].append({"line": start[0], "end_line": end[0],
                                       "function": sorted(info["functions"])[0]})
    files = []
    for file, e in sorted(per.items()):
        files.append({
            "file": file,
            "covered": e["covered"],
            "total": e["total"],
            "percent": round(100 * e["covered"] / e["total"]) if e["total"] else 100,
            "uncovered": e["uncovered"][:max_uncovered],
            "uncovered_omitted": max(0, len(e["uncovered"]) - max_uncovered),
        })
    covered = sum(f["covered"] for f in files)
    total = sum(f["total"] for f in files)
    reached = {normalize(f["file"]) for f in files}
    out = {
        "scope": "regions of the functions the harnesses reach; not the whole crate",
        "total": {"covered": covered, "regions": total, "percent": round(100 * covered / total) if total else 100},
        "files": files,
        "excluded_dependency_files": len(excluded),
    }
    if src_files:
        out["files_without_regions"] = sorted(f for f in src_files if normalize(f) not in reached
                                              and not any(normalize(f).endswith(r) or r.endswith(normalize(f))
                                                          for r in reached))
    return out


def compare(a, b, file_filter="", all_files=False):
    """Regions covered by exactly one of the two sets (only regions present in both)."""
    only_a, only_b = [], []
    for key in sorted(set(a) & set(b)):
        file, start, end = key
        if not wanted(file, file_filter, all_files) or a[key]["covered"] == b[key]["covered"]:
            continue
        entry = {"file": file, "line": start[0], "end_line": end[0], "function": sorted(a[key]["functions"])[0]}
        (only_a if a[key]["covered"] else only_b).append(entry)
    return {"covered_only_in_first": only_a, "covered_only_in_second": only_b}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", help="coverage run directories (`kanicov_*`)")
    parser.add_argument("--log", help="Kani's console output: use the coverage directories it names")
    parser.add_argument("--crate", help="use the latest coverage run under this crate or workspace")
    parser.add_argument("--file", default="", help="only files whose path contains this string (`../` is ignored)")
    parser.add_argument("--max-uncovered", type=int, default=20, help="uncovered regions listed per file")
    parser.add_argument("--compare", nargs="+", metavar="DIR", help="a second set of runs to compare with")
    parser.add_argument("--src", help="the crate's source directory, to list files no harness touched "
                        "(default: ./src when it exists)")
    parser.add_argument("--all-files", action="store_true", help="include dependency files")
    args = parser.parse_args(argv)
    dirs = list(args.dirs)
    try:
        if args.log:
            dirs += dirs_from_log(args.log)
        if args.crate:
            dirs += latest_run(args.crate)
    except OSError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    regions, n_files, bad = merge(dirs)
    if not n_files:
        print("error: no readable `*_kaniraw.json` coverage files found; run Kani with "
              "`-Z source-coverage --coverage` (in `--log`, look for the line "
              "'Coverage results saved to ...')" + (f"; unreadable: {bad}" if bad else ""), file=sys.stderr)
        return 1
    src = args.src or ("src" if os.path.isdir("src") else None)
    src_files = [os.path.relpath(f) for f in rust_files([src])] if src else []
    out = {"runs": dirs, "harness_files": n_files}
    if bad:
        out["unreadable_files"] = bad
        out["note"] = "unreadable raw files (empty or truncated, from a killed harness) were skipped"
    out.update(summarize(regions, args.file, args.max_uncovered, args.all_files, src_files))
    if args.compare:
        other, n_other, _ = merge(args.compare)
        if not n_other:
            print("error: no coverage files in the --compare directories", file=sys.stderr)
            return 1
        out["compare"] = compare(regions, other, args.file, args.all_files)
    emit(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
