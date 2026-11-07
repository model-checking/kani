#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Crate Verifiability Map: map a Rust workspace, plan verification, and track the results.

The map is a planning aid for choosing what to verify and in which order. It is not a proof
certificate: see `references/verifiability-map.md` for what each field means and what it cannot
tell you.

Commands:
    map PATH [--results FILE_OR_BUNDLE...] [--out map.json] [--callgraph auto|tool|text]
        Build the map of the Cargo workspace (or single .rs file) at PATH: functions, call graph,
        recursion, harnesses and contracts, and the status of each function from the Kani
        results given. The call graph comes from `cargo-callgraph` when it is installed
        (an optional external program, AGPL-3.0, never bundled) and from a text scan otherwise.
    update MAP --results FILE_OR_BUNDLE... [--out map.json]
        Merge new Kani results into a map, recomputing statuses, strategies, and priorities.
        Refuses to run when the sources changed since the map was built: map again.
    plan MAP [--goal FUNCTION]... [--out plan.json]
        A staged, bottom-up verification plan. Goals restrict it to the goal functions and what
        they depend on.
    explain MAP FUNCTION
        Everything the map knows about a function: evidence, strategy, uncertainty.
    report MAP [--max-nodes N] [--out graph.mmd]
        A Mermaid diagram of the call graph colored by verification status.

`--results` takes `--export-json` files and evidence bundle directories (see evidence.py).
Output is deterministic JSON: running the same command on the same sources and results gives
the same bytes.

Exit status: 0 on success, 2 when `update` finds the sources changed, 1 on input errors.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import verifiability_map as vm  # noqa: E402


def write(obj, out):
    text = json.dumps(obj, indent=2, sort_keys=True) + "\n"
    if out:
        Path(out).write_text(text, encoding="utf-8")
        print(json.dumps({"wrote": str(out)}))
    else:
        sys.stdout.write(text)


def strip_private(m):
    for f in m["functions"]:
        f.pop("_fn", None)
    return m


def load_map(path):
    m = json.loads(Path(path).read_text(encoding="utf-8"))
    if m.get("kind") != vm.KIND:
        raise ValueError(f"`{path}` is not a Crate Verifiability Map")
    major = m["schema_version"].split(".")[0]
    if major != vm.SCHEMA_VERSION.split(".")[0]:
        raise ValueError(f"unsupported map schema {m['schema_version']} (this tool reads {vm.SCHEMA_VERSION})")
    return m


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("map", help="build the map")
    p.add_argument("path", help="Cargo workspace directory or a single .rs file")
    p.add_argument("--results", nargs="*", default=[], help="--export-json files / evidence bundle directories")
    p.add_argument("--callgraph", choices=["auto", "tool", "text"], default="auto",
                   help="auto: cargo-callgraph if installed; tool: require it; text: text scan only")
    p.add_argument("--callgraph-json", help="use this cargo-callgraph JSON file instead of running the tool")
    p.add_argument("--timeout", type=int, default=900, help="seconds allowed for cargo-callgraph")
    p.add_argument("--out", help="write the map here (default: stdout)")
    p = sub.add_parser("update", help="merge new results into a map")
    p.add_argument("map")
    p.add_argument("--results", nargs="+", required=True)
    p.add_argument("--root", help="workspace root (default: the directory of the map's `root`, i.e. the current one)")
    p.add_argument("--out", help="write the map here (default: overwrite MAP)")
    p = sub.add_parser("plan", help="staged verification plan")
    p.add_argument("map")
    p.add_argument("--goal", action="append", default=[], help="function id, path, or name (repeatable)")
    p.add_argument("--out")
    p = sub.add_parser("explain", help="explain one function")
    p.add_argument("map")
    p.add_argument("function")
    p = sub.add_parser("report", help="Mermaid diagram")
    p.add_argument("map")
    p.add_argument("--max-nodes", type=int, default=60)
    p.add_argument("--out")
    args = parser.parse_args(argv)
    try:
        if args.command == "map":
            m = vm.build_map(args.path, args.results, args.callgraph, args.callgraph_json, args.timeout)
            write(strip_private(m), args.out)
            return 0
        m = load_map(args.map)
        if args.command == "update":
            root = Path(args.root) if args.root else Path.cwd()
            stale = vm.check_fresh(m, root)
            if stale:
                print("error: the sources changed since the map was built; run `map` again. First: "
                      f"{stale[0][0]} ({stale[0][1]})", file=sys.stderr)
                return 2
            write(vm.finalize(m, root, args.results), args.out or args.map)
        elif args.command == "plan":
            write(vm.plan(m, args.goal), args.out)
        elif args.command == "explain":
            write(vm.explain(m, args.function), None)
        elif args.command == "report":
            text = vm.mermaid(m, args.max_nodes)
            if args.out:
                Path(args.out).write_text(text, encoding="utf-8")
                print(json.dumps({"wrote": args.out}))
            else:
                sys.stdout.write(text)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.TimeoutExpired) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
