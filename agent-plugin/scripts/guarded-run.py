#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Run a command (usually `cargo kani ...`) under the plugin's memory guard.

The run happens in its own cgroup (systemd scope, on Linux) or is watched and killed by a
watchdog (elsewhere), so that a runaway CBMC ends only this run and never the shell or agent that
started it. Output is passed through as it arrives. At the end, one line on stderr says what the
run used, and a stopped run says why: `killed: memory (peak 9120 MB, limit 8192 MB) after 41.2s`.

Usage:
    guarded-run.py [--memory-limit-mb N] [--timeout-s N] [--export FILE] [--harness NAME]
                   [--log FILE] [--record FILE] -- cargo kani -Z export-json --export-json FILE ...

--memory-limit-mb   cap in MB (0: none). Default: `KANI_PLUGIN_MEMORY_MB`, else half of the memory
                    available to this process.
--timeout-s         wall-clock limit.
--export FILE       the `--export-json` file of the command: a completed run gets its peak memory
                    and CPU time added (`plugin_run`); a killed run, which writes none, gets an
                    export with `run_state` KILLED or TIMEOUT, so the result is not lost.
--harness NAME      the harness that runs, named in that export.

Exit status: the command's; 137 when it was killed for memory, 124 on a timeout.
"""

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import kani_env  # noqa: E402
from kani_run import annotate_export, run_guarded, write_stopped_export  # noqa: E402


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if "--" not in argv:
        print("error: put the command after `--`", file=sys.stderr)
        return 1
    split = argv.index("--")
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--memory-limit-mb", help="memory cap in MB (0 for none)")
    parser.add_argument("--timeout-s", type=float)
    parser.add_argument("--export")
    parser.add_argument("--harness")
    parser.add_argument("--log")
    parser.add_argument("--record", help="write the resource record (JSON) here")
    args = parser.parse_args(argv[:split])
    cmd = argv[split + 1:]
    if not cmd:
        print("error: no command after `--`", file=sys.stderr)
        return 1
    result = run_guarded(cmd, env=kani_env(), timeout_s=args.timeout_s, memory_mb=args.memory_limit_mb,
                         log_path=args.log, echo=sys.stdout.buffer)
    if args.export:
        if not annotate_export(args.export, result):
            write_stopped_export(args.export, result, args.harness)
    record = result.record()
    if args.record:
        with open(args.record, "w", encoding="utf-8") as f:
            json.dump(record, f, indent=2, sort_keys=True)
    peak = record["peak_memory_mb"]
    cpu = record["cpu_s"]
    summary = result.killed_message() or (
        f"finished in {record['elapsed_s']}s, peak {'unknown' if peak is None else f'{peak} MB'}, "
        f"cpu {'unknown' if cpu is None else f'{cpu}s'}")
    print(f"guarded-run: {summary} [{result.method} guard, limit {record['limit_mb']} MB]", file=sys.stderr)
    if result.killed == "memory":
        return 137
    if result.killed == "timeout":
        return 124
    return result.returncode if result.returncode >= 0 else 128 - result.returncode


if __name__ == "__main__":
    sys.exit(main())
