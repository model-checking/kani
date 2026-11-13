#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Run Kani harnesses one at a time, keeping every result as it finishes.

One `cargo kani` run over many harnesses writes its `--export-json` only at the very end: if the
run is killed (a tool time limit, the memory watchdog, a closed terminal) every result is lost,
even though most harnesses had finished. This script runs each harness in its own `cargo kani
--harness NAME --exact` process, with its own export, log, time limit, and memory cap, and merges
the results into one export (`export.json`, in the same format as `--export-json`) that it rewrites
after every harness. A harness that times out or exceeds the memory cap is recorded as TIMEOUT or
OUT_OF_MEMORY, and the run goes on.

Usage:
    run-harnesses.py [PATH] [--harness FILTER]... [--timeout-per SECONDS] [--total SECONDS]
                     [--mem-gb N] [--jobs N] [--out DIR] [--features F]... [--kani-arg ARG]...

Harnesses are the ones `cargo kani list` reports; `--harness` selects those whose name contains a
filter. `--timeout-per` (default 900) is wall-clock time per harness; `--total` (default 3600)
stops the run before it starts another harness. Each harness runs under the memory guard
(`kani_run.py`): in its own cgroup where systemd allows it, otherwise watched and killed; a harness
that exceeds `--memory-limit-mb` is killed alone and recorded as OUT_OF_MEMORY with its peak. The
default cap is half of the available memory (shared between `--jobs N` harnesses that run at once,
each can use several GB); each result records its peak memory and CPU time (`plugin_run`). Results go to `--out` (default: `<plugin output>/runs/<time>`): `export.json`
(merged, usable by summarize-results.py, check-vacuity.py, evidence.py), and one log and export
per harness. Progress is printed to stderr. Run it in the background for long jobs, and read
`export.json` while it runs.

Exit status: 0 if every selected harness succeeded, 2 if any did not or was not run, 1 on errors.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_export import ExportError, load_export  # noqa: E402
from kani_plugin_lib import export_args, find_root, join_option_values, kani_env, plugin_out_dir  # noqa: E402
from kani_run import annotate_export, default_limit_mb, run_guarded, safe_name, write_stopped_export  # noqa: E402

LIST = Path(__file__).resolve().parent / "list-harnesses.py"


def detected_features(package):
    """Unstable Kani features the sources use (`-Z function-contracts`, ...), as list-harnesses.py does."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("list_harnesses", LIST)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.detect_features(package)


def list_harness_names(path):
    out = subprocess.run([sys.executable, str(LIST), str(path), "--full"], capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(f"cannot list harnesses: {out.stdout[-800:] or out.stderr[-800:]}")
    listing = json.loads(out.stdout)
    names = []
    for key in ("standard-harnesses", "contract-harnesses"):
        for hs in listing.get(key, {}).values():
            names.extend(hs)
    return sorted(set(names))


def run_one(name, cwd, out_dir, kani_args, timeout, mem_mb):
    """Run one harness under the memory guard. Returns (verdict kind, export document or None,
    seconds, resource record). A harness that is stopped gets an export that says so."""
    safe = safe_name(name)
    export, log = out_dir / f"{safe}.json", out_dir / f"{safe}.log"
    cmd = ["cargo", "kani", "--harness", name, "--exact", *export_args(export), *kani_args]
    result = run_guarded(cmd, cwd=cwd, env=kani_env(), timeout_s=timeout, memory_mb=mem_mb, log_path=log)
    record = result.record()
    took = round(result.elapsed_s, 1)
    if result.killed:
        write_stopped_export(export, result, name)
        return ("OUT_OF_MEMORY" if result.killed == "memory" else "TIMEOUT"), None, took, record
    if export.exists():
        try:
            annotate_export(export, result)
            return None, load_export(export, workspace=cwd), took, record
        except ExportError:
            pass
    return "CRASHED", None, took, record


def synthetic_entry(name, kind, seconds, record=None):
    return {"name": name, "plugin_run": record, "crate_name": "", "file": "", "line": 0, "outcome": {"kind": kind},
            "resources": {"verification_time_s": seconds}, "failed_properties": [], "checks": {}, "covers": {},
            "attributes": {}, "is_automatically_generated": False, "is_bounded": False,
            "generated_concrete_test": False}


def write_merged(path, entries, first_doc, started, run_state):
    verdicts = {}
    for e in entries:
        o = e["outcome"]
        v = o.get("verdict") if o.get("kind") == "COMPLETED" else o.get("kind")
        verdicts[v] = verdicts.get(v, 0) + 1
    doc = {k: v for k, v in (first_doc or {}).items() if k not in ("harnesses", "summary", "run_state", "started_at")}
    doc.update({"run_state": run_state, "started_at": started, "harnesses": entries,
                "summary": {"total": len(entries), "successful": verdicts.get("SUCCESS", 0),
                            "failed": verdicts.get("FAILURE", 0)}})
    tmp = Path(str(path) + ".tmp")
    tmp.write_text(json.dumps(doc, indent=2) + "\n")
    tmp.replace(path)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("path", nargs="?", default=".")
    parser.add_argument("--harness", action="append", default=[], help="select harnesses whose name contains this")
    parser.add_argument("--timeout-per", type=int, default=900, help="seconds per harness (default 900)")
    parser.add_argument("--total", type=int, default=3600, help="seconds for the whole run (default 3600)")
    parser.add_argument("--mem-gb", type=float, default=0, help="memory cap per harness in GB (same as "
                        "--memory-limit-mb)")
    parser.add_argument("--memory-limit-mb", type=float, help="memory cap per harness in MB (0: none). Default: "
                        "KANI_PLUGIN_MEMORY_MB, else half of the available memory shared by --jobs")
    parser.add_argument("--jobs", type=int, default=1, help="harnesses to run at once (default 1)")
    parser.add_argument("--out", help="output directory")
    parser.add_argument("--features", action="append", default=[], help="cargo features (repeatable)")
    parser.add_argument("--kani-arg", action="append", default=[], help="extra Kani argument, e.g. "
                        "`--kani-arg -Z --kani-arg function-contracts` (repeatable)")
    args = parser.parse_args(join_option_values(argv, {"--kani-arg"}))
    path = Path(args.path).resolve()
    root, package = find_root(path)
    if package is None:
        print("error: not a Cargo project", file=sys.stderr)
        return 1
    kani_args = list(args.kani_arg)
    for f in args.features:
        kani_args += ["--features", f]
    given = {kani_args[i + 1] for i, a in enumerate(kani_args[:-1]) if a == "-Z"}
    for feature in detected_features(package):
        if feature not in given:
            kani_args += ["-Z", feature]
    kani_args += ["-Z", "unstable-options", "--harness-timeout", f"{max(args.timeout_per - 30, 30)}s"]
    try:
        names = list_harness_names(package)
    except RuntimeError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    if args.harness:
        names = [n for n in names if any(f in n for f in args.harness)]
        if not names:
            print(f"error: no harness contains any of {args.harness}", file=sys.stderr)
            return 1
    out_dir = Path(args.out) if args.out else plugin_out_dir(root) / "runs" / time.strftime("%Y%m%dT%H%M%S")
    out_dir.mkdir(parents=True, exist_ok=True)
    merged = out_dir / "export.json"
    started = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    t0 = time.time()
    lock = threading.Lock()
    entries, first_doc, not_run = [], [None], []
    mem_mb = args.memory_limit_mb if args.memory_limit_mb is not None else (args.mem_gb * 1024 or None)
    if mem_mb is None and args.jobs > 1 and os.environ.get("KANI_PLUGIN_MEMORY_MB") is None:
        # The default cap is half of the available memory: share it between the harnesses that run at once.
        default = default_limit_mb()
        mem_mb = default // args.jobs if default else None

    def work(name):
        if time.time() - t0 > args.total:
            with lock:
                not_run.append(name)
            return
        kind, doc, took, record = run_one(name, package, out_dir, kani_args, args.timeout_per, mem_mb)
        with lock:
            if doc is not None:
                first_doc[0] = first_doc[0] or doc
                new = doc["harnesses"] or [synthetic_entry(name, "CRASHED", took, record)]
                for entry in new:
                    entry["plugin_run"] = record
            else:
                new = [synthetic_entry(name, kind, took, record)]
            entries.extend(new)
            verdict = [(e["outcome"].get("verdict") or e["outcome"]["kind"]) for e in new]
            print(f"[{len(entries) + len(not_run)}/{len(names)}] {name}: {', '.join(verdict)} ({took}s)",
                  file=sys.stderr, flush=True)
            if "CRASHED" in verdict:
                tail = (out_dir / (re.sub(r"[^A-Za-z0-9_.-]", "_", name) + ".log")).read_text(errors="replace")
                print("    " + "\n    ".join(tail.strip().splitlines()[-3:]), file=sys.stderr, flush=True)
            write_merged(merged, sorted(entries, key=lambda e: e["name"]), first_doc[0], started, "PARTIAL")

    with ThreadPoolExecutor(max_workers=max(args.jobs, 1)) as pool:
        list(pool.map(work, names))
    write_merged(merged, sorted(entries, key=lambda e: e["name"]), first_doc[0], started,
                 "PARTIAL" if not_run else "COMPLETE")
    verdicts = {}
    for e in entries:
        v = e["outcome"].get("verdict") or e["outcome"]["kind"]
        verdicts[v] = verdicts.get(v, 0) + 1
    print(json.dumps({"export": str(merged), "out_dir": str(out_dir), "run_state": "PARTIAL" if not_run else "COMPLETE",
                      "verdicts": dict(sorted(verdicts.items())), "not_run": sorted(not_run),
                      "next": f"summarize-results.py {merged}"}, indent=2, sort_keys=True))
    return 0 if verdicts == {"SUCCESS": len(entries)} and not not_run else 2


if __name__ == "__main__":
    sys.exit(main())
