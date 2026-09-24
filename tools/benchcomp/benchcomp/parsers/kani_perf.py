# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import os
import pathlib
import re
import textwrap

import yaml

import benchcomp.parsers


def get_description():
    return textwrap.dedent("""\
        Read Kani and CBMC statistics from the expected.out files of the kani
        perf regression suite.
        """)


def _get_metrics():
    return {
        "verification_time": {
            # Letter 'e' and hyphen handle scientific notation
            "pat": re.compile(r"Verification Time: (?P<value>[-e\d\.]+)s"),
            "parse": float,
        },
        "solver_runtime": {
            "pat": re.compile(r"Runtime Solver: (?P<value>[-e\d\.]+)s"),
            "parse": float,
        },
        # CBMC solves a harness in several calls, and the number of calls is not stable across
        # runs of the same code (https://github.com/model-checking/kani/issues/4821). Counting
        # them makes that visible, and turns `solver_runtime` -- a sum over a varying number of
        # calls -- into something comparable: see `solver_runtime_per_call` below.
        "solver_calls": {
            "pat": re.compile(r"Solving with (?P<value>.+)"),
            "parse": lambda _: 1,
        },
        "removed_program_steps": {
            "pat": re.compile(r"slicing removed (?P<value>\d+) assignments"),
            "parse": int,
        },
        "number_program_steps": {
            "pat": re.compile(r"size of program expression: (?P<value>\d+) steps"),
            "parse": int,
        },
        "number_vccs": {
            "pat": re.compile(
                r"Generated \d+ VCC\(s\), (?P<value>\d+) remaining after simplification"),
            "parse": int,
        },
        "symex_runtime": {
            "pat": re.compile(r"Runtime Symex: (?P<value>[-e\d\.]+)s"),
            "parse": float,
        },
        "success": {
            "pat": re.compile(r"VERIFICATION:- (?P<value>\w+)"),
            "parse": lambda v: v == "SUCCESSFUL",
        },
    }


def get_metrics():
    metrics = dict(_get_metrics())
    for metric, info in metrics.items():
        for field in ("pat", "parse"):
            info.pop(field)

    # This is not a metric we return; it is used to find the correct value for
    # the number_program_steps metric
    metrics.pop("removed_program_steps", None)

    # Derived in `main` from `solver_runtime` and `solver_calls` rather than parsed from a line,
    # so it has no pattern to strip.
    metrics["solver_runtime_per_call"] = {}

    return metrics


def main(root_dir):
    benchmarks = {}
    test_out_dir = root_dir / "build" / "tests" / "perf"
    harness_pat = re.compile(r"Checking harness (?P<name>.+)\.\.\.")

    metrics = _get_metrics()
    for out_file in pathlib.Path(test_out_dir).rglob("expected.out"):
        test_name = str(out_file.parent.parent.relative_to(test_out_dir))
        with open(out_file) as handle:
            for line in handle:
                # Each outfile contains output from multiple harnesses
                m = harness_pat.match(line)
                if m:
                    bench_name = f"{test_name}/{m['name']}"
                    benchmarks[bench_name] = {"metrics": {}}
                    continue

                for metric, metric_info in metrics.items():
                    m = metric_info["pat"].match(line)
                    if not m:
                        continue

                    parse = metric_info["parse"]
                    try:
                        # CBMC prints out some metrics more than once, e.g.
                        # "Solver" and "decision procedure". Add those
                        # values together
                        benchmarks[bench_name]["metrics"][metric] += parse(m["value"])
                    except (KeyError, TypeError):
                        benchmarks[bench_name]["metrics"][metric] = parse(m["value"])
                    break

    for bench_name, bench_info in benchmarks.items():
        try:
            n_steps = bench_info["metrics"]["number_program_steps"]
            rm_steps = bench_info["metrics"]["removed_program_steps"]
            bench_info["metrics"]["number_program_steps"] = n_steps - rm_steps
            bench_info["metrics"].pop("removed_program_steps", None)
        except KeyError:
            pass

        try:
            calls = bench_info["metrics"]["solver_calls"]
            runtime = bench_info["metrics"]["solver_runtime"]
            bench_info["metrics"]["solver_runtime_per_call"] = runtime / calls
        except (KeyError, ZeroDivisionError):
            pass

    return {
        "metrics": get_metrics(),
        "benchmarks": benchmarks,
    }


if __name__ == "__main__":
    try:
        result = main(os.getcwd())
        print(yaml.dump(result, default_flow_style=False))
    except BaseException:
        print(yaml.dump(
            benchcomp.parsers.get_empty_parser_result(),
            default_flow_style=False))
