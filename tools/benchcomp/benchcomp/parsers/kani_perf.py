# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import pathlib
import textwrap
import re


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

    return {
        "metrics": get_metrics(),
        "benchmarks": benchmarks,
    }
