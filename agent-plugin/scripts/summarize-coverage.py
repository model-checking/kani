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
    summarize-coverage.py DIR... [--compare DIR...]

`--compare` takes another set of runs (for example before and after adding a harness) and
reports the regions covered only by one of the two sets.

Regions inside the harnesses themselves (for example an error branch of the harness that
never runs) are reported too: a dead branch in a harness is a gap in the proof. Use `--file`
to focus on the code under verification.

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
from kani_plugin_lib import emit  # noqa: E402

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


def merge(dirs):
    """(file, start, end, function) -> covered, merged over all raw files in `dirs`."""
    regions = {}
    files = 0
    for d in dirs:
        for f in glob.glob(os.path.join(d, "*_kaniraw.json")):
            files += 1
            with open(f, encoding="utf-8") as fh:
                data = json.load(fh).get("data", {})
            for file, rs in data.items():
                for r in rs:
                    key = (file, tuple(r["region"]["start"]), tuple(r["region"]["end"]), r["function"])
                    regions[key] = regions.get(key, False) or r["status"] == "COVERED"
    return regions, files


def summarize(regions, file_filter="", max_uncovered=20):
    per = collections.defaultdict(lambda: {"covered": 0, "total": 0, "uncovered": []})
    for (file, start, end, function), covered in sorted(regions.items()):
        if file_filter not in file:
            continue
        entry = per[file]
        entry["total"] += 1
        entry["covered"] += covered
        if not covered:
            entry["uncovered"].append({"line": start[0], "end_line": end[0], "function": function})
    files = []
    for file, e in sorted(per.items()):
        item = {
            "file": file,
            "covered": e["covered"],
            "total": e["total"],
            "percent": round(100 * e["covered"] / e["total"]) if e["total"] else 100,
        }
        if e["uncovered"]:
            item["uncovered"] = e["uncovered"][:max_uncovered]
            if len(e["uncovered"]) > max_uncovered:
                item["uncovered_omitted"] = len(e["uncovered"]) - max_uncovered
        files.append(item)
    covered = sum(f["covered"] for f in files)
    total = sum(f["total"] for f in files)
    return {
        "total": {"covered": covered, "regions": total, "percent": round(100 * covered / total) if total else 100},
        "files": files,
    }


def compare(a, b, file_filter=""):
    """Regions covered by exactly one of the two sets (only regions present in both)."""
    only_a, only_b = [], []
    for key in sorted(set(a) & set(b)):
        file, start, end, function = key
        if file_filter not in file or a[key] == b[key]:
            continue
        entry = {"file": file, "line": start[0], "end_line": end[0], "function": function}
        (only_a if a[key] else only_b).append(entry)
    return {"covered_only_in_first": only_a, "covered_only_in_second": only_b}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("dirs", nargs="*", help="coverage run directories (`kanicov_*`)")
    parser.add_argument("--log", help="Kani's console output: use the coverage directories it names")
    parser.add_argument("--crate", help="use the latest coverage run under this crate or workspace")
    parser.add_argument("--file", default="", help="only files whose path contains this string")
    parser.add_argument("--max-uncovered", type=int, default=20, help="uncovered regions listed per file")
    parser.add_argument("--compare", nargs="+", metavar="DIR", help="a second set of runs to compare with")
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
    regions, n_files = merge(dirs)
    if not n_files:
        print("error: no `*_kaniraw.json` coverage files found; run Kani with "
              "`-Z source-coverage --coverage` (in `--log`, look for the line "
              "'Coverage results saved to ...')", file=sys.stderr)
        return 1
    out = {"runs": dirs, "harness_files": n_files}
    out.update(summarize(regions, args.file, args.max_uncovered))
    if args.compare:
        other, n_other = merge(args.compare)
        if not n_other:
            print("error: no coverage files in the --compare directories", file=sys.stderr)
            return 1
        out["compare"] = compare(regions, other, args.file)
    emit(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
