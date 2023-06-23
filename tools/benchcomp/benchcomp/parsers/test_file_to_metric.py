# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# This parser is used by the test suite. It reads files in a directory of
# directories. Each directory represents a benchmark name; inside that
# directory, every file name is the name of a metric and the contents of the
# file are the metric value. This is to allow writing ad-hoc regression tests
# without actually running a real benchmark suite.


import json
import pathlib


def main(root_dir):
    ret = {
        "metrics": {},
        "benchmarks": {},
    }
    for benchmark in pathlib.Path(root_dir).iterdir():
        ret["benchmarks"][benchmark.name] = {"metrics": {}}
        for metric in pathlib.Path(benchmark).iterdir():
            ret["metrics"][metric.name] = {}
            with open(metric) as handle:
                value = json.loads(handle.read().strip())
            ret["benchmarks"][benchmark.name]["metrics"][metric.name] = value
    return ret
